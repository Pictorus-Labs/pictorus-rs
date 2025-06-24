use core::marker::PhantomData;

use pictorus_block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock};

use crate::traits::{Float, MatrixOps};

/// Performs a 1D lookup against a set of break points and data points.
///
/// The lookup can either be performed using linear interpolation or nearest neighbor
/// interpolation, depending on the `interp_method` parameter. For matrix inputs, the
/// lookup is performed element-wise.
pub struct Lookup1DBlock<const N: usize, S, T>
where
    S: Float,
    T: Apply<N, S>,
{
    pub data: OldBlockData,
    buffer: T,
    _unused: PhantomData<S>,
}

impl<const N: usize, S: Float, T: Apply<N, S>> ProcessBlock for Lookup1DBlock<N, S, T>
where
    OldBlockData: FromPass<T>,
{
    type Inputs = T;
    type Output = T;
    type Parameters = Parameters<N, S>;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) -> pictorus_traits::PassBy<'b, Self::Output> {
        let output = T::apply(&mut self.buffer, inputs, parameters);
        self.data = OldBlockData::from_pass(output);
        output
    }
}

impl<const N: usize, S: Float, T: Apply<N, S>> Default for Lookup1DBlock<N, S, T>
where
    OldBlockData: FromPass<T>,
{
    fn default() -> Self {
        Self {
            data: <OldBlockData as FromPass<T>>::from_pass(T::default().as_by()),
            buffer: T::default(),
            _unused: PhantomData,
        }
    }
}

#[derive(strum::EnumString)]
pub enum InterpMethod {
    /// Linear interpolation
    Linear,
    /// Nearest neighbor interpolation
    Nearest,
}

/// Parameters for the Lookup1DBlock
pub struct Parameters<const N: usize, S: Float> {
    /// Interpolation method to use
    interp_method: InterpMethod,
    /// Break points for the lookup
    break_points_u1: [S; N],
    /// Data points for the lookup
    data_points: [S; N],
}

impl<const N: usize, S: Float> Parameters<N, S> {
    pub fn new(
        interp_method: &str,
        break_points_u1: &OldBlockData,
        data_points: &OldBlockData,
    ) -> Self {
        let mut break_points_u1_arr = [S::default(); N];
        for (i, val) in break_points_u1.iter().enumerate() {
            break_points_u1_arr[i] = S::from(*val).expect("Failed to convert break point to float");
        }

        let mut data_points_arr = [S::default(); N];
        for (i, val) in data_points.iter().enumerate() {
            data_points_arr[i] = S::from(*val).expect("Failed to convert data point to float");
        }

        Self {
            interp_method: interp_method
                .parse()
                .expect("Invalid interp method. Must be Linear or Nearest"),
            break_points_u1: break_points_u1_arr,
            data_points: data_points_arr,
        }
    }
}

pub trait Apply<const N: usize, S: Float>: Pass + Default {
    fn apply<'s>(
        store: &'s mut Self,
        input: PassBy<Self>,
        params: &Parameters<N, S>,
    ) -> PassBy<'s, Self>;
}

impl<const N: usize, S: Float> Apply<N, S> for S {
    fn apply<'s>(
        _store: &'s mut Self,
        input: PassBy<Self>,
        params: &Parameters<N, S>,
    ) -> PassBy<'s, Self> {
        let interp_method = &params.interp_method;

        if input < params.break_points_u1[0] {
            return params.data_points[0];
        } else if input >= params.break_points_u1[N - 1] {
            return params.data_points[N - 1];
        }

        match interp_method {
            InterpMethod::Linear => linear_interpolation(input, params),
            InterpMethod::Nearest => nearest_interpolation(input, params),
        }
    }
}

impl<const N: usize, const NROWS: usize, const NCOLS: usize, S: Float> Apply<N, S>
    for Matrix<NROWS, NCOLS, S>
{
    fn apply<'s>(
        store: &'s mut Self,
        input: PassBy<Self>,
        params: &Parameters<N, S>,
    ) -> PassBy<'s, Self> {
        // For the scalar case the store isn't actually used, but is required by the trait
        let mut dummy_store = S::default();
        input.for_each(|v, c, r| {
            store.data[c][r] = S::apply(&mut dummy_store, v, params);
        });
        store.as_by()
    }
}

fn linear_interpolation<const N: usize, S: Float>(
    lookup_point_val: S,
    params: &Parameters<N, S>,
) -> S {
    let mut idx: usize = 0;
    for (i, break_point) in params.break_points_u1.iter().enumerate() {
        if lookup_point_val < *break_point {
            idx = i;
            break;
        }
    }

    let k = (lookup_point_val - params.break_points_u1[idx - 1])
        / (params.break_points_u1[idx] - params.break_points_u1[idx - 1]);
    params.data_points[idx - 1] + k * (params.data_points[idx] - params.data_points[idx - 1])
}

fn nearest_interpolation<const N: usize, S: Float>(
    lookup_point_val: S,
    params: &Parameters<N, S>,
) -> S {
    let mut idx: usize = 0;
    for (i, break_point) in params.break_points_u1.iter().enumerate() {
        if lookup_point_val < *break_point {
            idx = i;
            break;
        }
    }
    let delt_high = params.break_points_u1[idx] - lookup_point_val;
    let delt_low = lookup_point_val - params.break_points_u1[idx - 1];

    match delt_high > delt_low {
        true => params.data_points[idx - 1],
        false => params.data_points[idx],
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::StubContext;
    #[cfg(feature = "tricore")]
    use pictorus_traits::FlattenSlice as _;

    use super::*;

    #[test]
    fn test_scalar_linear() {
        let ctxt = StubContext::default();
        let break_points_u1 = OldBlockData::from_vector(&[0.0, 1.0, 2.0]);
        let data_points = OldBlockData::from_vector(&[-1.0, 1.0, 10.0]);
        let params = Parameters::new("Linear", &break_points_u1, &data_points);

        let mut block = Lookup1DBlock::<3, f64, f64>::default();
        let res = block.process(&params, &ctxt, 0.0);
        assert_eq!(res, -1.0);
        assert_eq!(block.data.scalar(), -1.0);

        let res = block.process(&params, &ctxt, 1.0);
        assert_eq!(res, 1.0);
        assert_eq!(block.data.scalar(), 1.0);

        let res = block.process(&params, &ctxt, 0.5);
        assert_eq!(res, 0.0);
        assert_eq!(block.data.scalar(), 0.0);

        let res = block.process(&params, &ctxt, 1.5);
        let expected = 11.0 / 2.0;
        assert_eq!(res, expected);
        assert_eq!(block.data.scalar(), expected);

        // Verify clamps output
        let res = block.process(&params, &ctxt, 3.0);
        assert_eq!(res, 10.0);
        assert_eq!(block.data.scalar(), 10.0);

        let res = block.process(&params, &ctxt, -100.0);
        assert_eq!(res, -1.0);
        assert_eq!(block.data.scalar(), -1.0);
    }

    #[test]
    fn test_scalar_nearest() {
        let ctxt = StubContext::default();
        let break_points_u1 = OldBlockData::from_vector(&[0.0, 1.0, 2.0]);
        let data_points = OldBlockData::from_vector(&[-1.0, 1.0, 10.0]);
        let params = Parameters::new("Nearest", &break_points_u1, &data_points);

        let mut block = Lookup1DBlock::<3, f64, f64>::default();
        let res = block.process(&params, &ctxt, 0.0);
        assert_eq!(res, -1.0);
        assert_eq!(block.data.scalar(), -1.0);

        let res = block.process(&params, &ctxt, 0.25);
        assert_eq!(res, -1.0);
        assert_eq!(block.data.scalar(), -1.0);

        let res = block.process(&params, &ctxt, 0.5);
        assert_eq!(res, 1.0);
        assert_eq!(block.data.scalar(), 1.0);

        let res = block.process(&params, &ctxt, 0.75);
        assert_eq!(res, 1.0);
        assert_eq!(block.data.scalar(), 1.0);

        let res = block.process(&params, &ctxt, 1.75);
        assert_eq!(res, 10.0);
        assert_eq!(block.data.scalar(), 10.0);

        // Verify clamps output
        let res = block.process(&params, &ctxt, 3.0);
        assert_eq!(res, 10.0);
        assert_eq!(block.data.scalar(), 10.0);

        let res = block.process(&params, &ctxt, -100.0);
        assert_eq!(res, -1.0);
        assert_eq!(block.data.scalar(), -1.0);
    }

    #[test]
    fn test_matrix_linear() {
        let ctxt = StubContext::default();
        let break_points_u1 = OldBlockData::from_vector(&[0.0, 1.0, 2.0]);
        let data_points = OldBlockData::from_vector(&[-1.0, 1.0, 10.0]);
        let params = Parameters::new("Linear", &break_points_u1, &data_points);

        let mut block = Lookup1DBlock::<3, f64, Matrix<2, 2, f64>>::default();
        let input = Matrix {
            data: [[0.0, 1.0], [0.5, 1.5]],
        };
        let res = block.process(&params, &ctxt, &input);
        let expected = Matrix {
            data: [[-1.0, 1.0], [0.0, 11.0 / 2.0]],
        };
        assert_eq!(res.data, expected.data);
        assert_eq!(
            block.data.get_data().as_slice(),
            expected.data.as_flattened()
        );

        // Verify clamps output
        let input = Matrix {
            data: [[3.0, 300.0], [-100.0, -10000.0]],
        };
        let res = block.process(&params, &ctxt, &input);
        let expected = Matrix {
            data: [[10.0, 10.0], [-1.0, -1.0]],
        };
        assert_eq!(res.data, expected.data);
        assert_eq!(
            block.data.get_data().as_slice(),
            expected.data.as_flattened()
        );
    }

    #[test]
    fn test_matrix_nearest() {
        let ctxt = StubContext::default();
        let break_points_u1 = OldBlockData::from_vector(&[0.0, 1.0, 2.0]);
        let data_points = OldBlockData::from_vector(&[-1.0, 1.0, 10.0]);
        let params = Parameters::new("Nearest", &break_points_u1, &data_points);

        let mut block = Lookup1DBlock::<3, f64, Matrix<2, 2, f64>>::default();
        let input = Matrix {
            data: [[0.0, 0.25], [0.5, 1.75]],
        };
        let res = block.process(&params, &ctxt, &input);
        let expected = Matrix {
            data: [[-1.0, -1.0], [1.0, 10.0]],
        };
        assert_eq!(res.data, expected.data);
        assert_eq!(
            block.data.get_data().as_slice(),
            expected.data.as_flattened()
        );

        // Verify clamps output
        let input = Matrix {
            data: [[3.0, 300.0], [-100.0, -10000.0]],
        };
        let res = block.process(&params, &ctxt, &input);
        let expected = Matrix {
            data: [[10.0, 10.0], [-1.0, -1.0]],
        };
        assert_eq!(res.data, expected.data);
        assert_eq!(
            block.data.get_data().as_slice(),
            expected.data.as_flattened()
        );
    }
}
