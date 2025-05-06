use crate::nalgebra_interop::MatrixExt;
use crate::traits::Float;
use block_data::{BlockData as OldBlockData, FromPass};
use core::time::Duration;
use pictorus_traits::{HasIc, Matrix, Pass, PassBy, ProcessBlock};

/// Block for applying an Infinite Impulse Response (IIR) filter to an input signal.
pub struct IirFilterBlock<T: Pass + Default>
where
    OldBlockData: FromPass<T>,
{
    pub data: OldBlockData,
    buffer: Option<T>,
}

impl<T: Pass + Default> Default for IirFilterBlock<T>
where
    OldBlockData: FromPass<T>,
{
    fn default() -> Self {
        IirFilterBlock {
            data: <OldBlockData as FromPass<T>>::from_pass(T::default().as_by()),
            buffer: None,
        }
    }
}

impl<T: Float> HasIc for IirFilterBlock<T>
where
    OldBlockData: FromPass<T>,
{
    fn new(parameters: &Self::Parameters) -> Self {
        IirFilterBlock::<T> {
            buffer: Some(parameters.ic),
            data: <OldBlockData as FromPass<T>>::from_pass(parameters.ic),
        }
    }
}

impl<T, const NROWS: usize, const NCOLS: usize> HasIc for IirFilterBlock<Matrix<NROWS, NCOLS, T>>
where
    T: Float,
    OldBlockData: FromPass<Matrix<NROWS, NCOLS, T>>,
{
    fn new(parameters: &Self::Parameters) -> Self {
        IirFilterBlock::<pictorus_traits::Matrix<NROWS, NCOLS, T>> {
            buffer: Some(parameters.ic),
            data: <OldBlockData as FromPass<Matrix<NROWS, NCOLS, T>>>::from_pass(&parameters.ic),
        }
    }
}

impl<T> ProcessBlock for IirFilterBlock<T>
where
    T: Float,
    OldBlockData: FromPass<T>,
{
    type Inputs = T;
    type Output = T;
    type Parameters = Parameters<T, T>;
    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        let timestep_s = T::from_duration(context.timestep().unwrap_or(Duration::from_secs(0)));
        let alpha = timestep_s / (timestep_s + parameters.time_constant_s);
        let last_val = self.buffer.unwrap_or(parameters.ic);
        let res = alpha * inputs + ((T::one() - alpha) * last_val);
        self.data = <OldBlockData as FromPass<T>>::from_pass(res);
        self.buffer.insert(res).as_by()
    }
}

impl<T, const NROWS: usize, const NCOLS: usize> ProcessBlock
    for IirFilterBlock<Matrix<NROWS, NCOLS, T>>
where
    T: Float,
    OldBlockData: FromPass<Matrix<NROWS, NCOLS, T>>,
{
    type Inputs = Matrix<NROWS, NCOLS, T>;
    type Output = Matrix<NROWS, NCOLS, T>;
    type Parameters = Parameters<Matrix<NROWS, NCOLS, T>, T>;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        let timestep_s = T::from_duration(context.timestep().unwrap_or(Duration::from_secs(0)));
        let alpha = timestep_s / (timestep_s + parameters.time_constant_s);
        let input = inputs.as_view();
        let last_val = self.buffer.as_ref().unwrap_or(&parameters.ic).as_view();
        let res = input * alpha + (last_val * (T::one() - alpha));
        let res = Self::Output::from_view(&res.as_view());
        self.data = OldBlockData::from_pass(&res);
        self.buffer.insert(res)
    }
}

/// Parameters for the IIR filter block.
pub struct Parameters<T, C: Float> {
    /// The time constant of the filter in seconds.
    pub time_constant_s: C,
    /// Initial condition to set the default state of the block.
    ic: T,
}

impl<T, C: Float> Parameters<T, C> {
    pub fn new(ic: T, time_constant_s: C) -> Self {
        Parameters {
            ic,
            time_constant_s,
        }
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use super::*;
    use crate::testing::StubContext;
    use approx::assert_relative_eq;

    #[test]
    fn test_iir_filter_block_scalar() {
        let mut ctxt = StubContext::new(Duration::from_secs(0), None, Duration::from_secs(1));
        // Use 1s settling time
        let time_constants_s = 1.0;
        let mut block = IirFilterBlock::<f64>::default();
        let parameters = Parameters::new(0.0, time_constants_s);

        // T = 0, timestep = None, therefore `alpha` = 0 and output = IC
        let res = block.process(&parameters, &ctxt, 1.0);
        assert_relative_eq!(res, 0.0, max_relative = 0.01);

        ctxt.time = Duration::from_secs(1);
        ctxt.timestep = Some(Duration::from_secs(1));

        // Sending in unity with 1s timestamps should result in filter reaching
        // roughly 50% of final data
        let res = block.process(&parameters, &ctxt, 1.0);
        assert_relative_eq!(res, 0.5, max_relative = 0.01);
        assert_relative_eq!(block.data.scalar(), 0.5, max_relative = 0.01);
    }

    #[test]
    fn test_iir_filter_block_matrix() {
        let mut ctxt = StubContext::new(Duration::from_secs(0), None, Duration::from_secs(1));
        // Use 1s settling time
        let time_constants_s = 1.0;
        let mut block = IirFilterBlock::<Matrix<1, 2, f64>>::default();
        let ic = Matrix {
            data: [[0.0], [0.0]],
        };
        let parameters = Parameters::new(ic, time_constants_s);
        let input = Matrix {
            data: [[1.0], [2.0]],
        };

        // T = 0, timestep = None, therefore `alpha` = 0 and output = IC
        let res = block.process(&parameters, &ctxt, &input);
        let expected = [[0.0], [0.0]];
        assert_relative_eq!(
            res.data.as_flattened(),
            expected.as_flattened(),
            max_relative = 0.01
        );

        ctxt.time = Duration::from_secs(1);
        ctxt.timestep = Some(Duration::from_secs(1));
        // Sending in unity with 1s timestamps should result in filter reaching
        // roughly 50% of final data

        let res = block.process(&parameters, &ctxt, &input);
        let expected = [[0.5, 1.0]];
        assert_relative_eq!(
            res.data.as_flattened(),
            expected.as_flattened(),
            max_relative = 0.01
        );
        assert_relative_eq!(
            block.data.get_data().as_slice(),
            expected.as_flattened(),
            max_relative = 0.01
        );
    }

    #[test]
    fn test_iir_filter_block_scalar_w_ic() {
        let mut ctxt = StubContext::new(Duration::from_secs(0), None, Duration::from_secs(1));
        // Use 1s settling time
        let time_constants_s = 1.0;
        let mut block = IirFilterBlock::<f64>::default();
        let parameters = Parameters::new(0.0, time_constants_s);

        // T = 0, timestep = None, therefore `alpha` = 0 and output = IC
        let res = block.process(&parameters, &ctxt, 1.0);
        assert_relative_eq!(res, 0.0, max_relative = 0.01);

        ctxt.time = Duration::from_secs(1);
        ctxt.timestep = Some(Duration::from_secs(1));

        // Sending in unity with 1s timestamps should result in filter reaching
        // roughly 50% of final data
        let res = block.process(&parameters, &ctxt, 1.0);
        assert_relative_eq!(res, 0.5, max_relative = 0.01);
        assert_relative_eq!(block.data.scalar(), 0.5, max_relative = 0.01);

        // Reset the block with a new initial condition
        let new_ic = 0.5;
        let parameters = Parameters::new(new_ic, time_constants_s);
        let res = block.process(&parameters, &ctxt, 1.0);
        assert_relative_eq!(res, 0.75, max_relative = 0.01);
        assert_relative_eq!(block.data.scalar(), 0.75, max_relative = 0.01);
    }

    #[test]
    fn test_iir_filter_block_matrix_w_ic() {
        let mut ctxt = StubContext::new(Duration::from_secs(0), None, Duration::from_secs(1));
        // Use 1s settling time
        let time_constants_s = 1.0;
        let mut block = IirFilterBlock::<Matrix<1, 2, f64>>::default();
        let ic = Matrix {
            data: [[0.5], [1.0]],
        };
        let parameters = Parameters::new(ic, time_constants_s);

        let input = Matrix {
            data: [[1.0], [2.0]],
        };

        // T = 0, timestep = None, therefore `alpha` = 0 and output = IC
        let res = block.process(&parameters, &ctxt, &input);
        let expected = [[0.5], [1.0]];
        assert_relative_eq!(
            res.data.as_flattened(),
            expected.as_flattened(),
            max_relative = 0.01
        );

        ctxt.time = Duration::from_secs(1);
        ctxt.timestep = Some(Duration::from_secs(1));

        let res = block.process(&parameters, &ctxt, &input);
        let expected = [[0.75, 1.5]];
        assert_relative_eq!(
            res.data.as_flattened(),
            expected.as_flattened(),
            max_relative = 0.01
        );
        assert_relative_eq!(
            block.data.get_data().as_slice(),
            expected.as_flattened(),
            max_relative = 0.01
        );
    }
}
