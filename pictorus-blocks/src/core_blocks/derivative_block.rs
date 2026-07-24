use crate::{matrix_ext::MatrixNalgebraExt, traits::Float};
use pictorus_traits::{HasIc, Matrix, Pass, ProcessBlock};

/// Compute the discrete derivative of a signal using a sliding window of samples.
pub struct DerivativeBlock<T: Pass + Default + Copy, const N: usize> {
    samples: [T; N],
    sample_index: usize,
    initial_accumulation: bool,
    output: T,
}

impl<const N: usize, T: Pass + Default + Copy> Default for DerivativeBlock<T, N> {
    fn default() -> Self {
        const {
            panic!(
                "DerivativeBlock has initial conditions and must be constructed with \
                 DerivativeBlock::new(&parameters) (HasIc trait), not Default::default()."
            )
        }
    }
}

impl<T: Float, const N: usize> ProcessBlock for DerivativeBlock<T, N> {
    type Inputs = T;
    type Output = T;
    type Parameters = Parameters<T>;

    fn process<'b>(
        &'b mut self,
        _parameters: &Self::Parameters,
        context: &dyn pictorus_traits::Context,
        inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) -> pictorus_traits::PassBy<'b, Self::Output> {
        // store the current input in the sample buffer
        self.samples[self.sample_index] = inputs;

        // increment the sample index, wrapping at N (and setting initial_accumulation to false)
        self.sample_index += 1;
        if self.sample_index >= N {
            self.sample_index = 0;
            self.initial_accumulation = false;
        }

        // Only set the output when initial accumulation is done, otherwise use the IC
        if !self.initial_accumulation {
            self.output = (inputs - self.samples[self.sample_index])
                / ((T::from_usize(N).unwrap() - T::one())
                    * T::from_duration(context.timestep().expect(
                        "timestep should never be None outside of Initial Accumulation phase",
                    )));
        }

        self.output.as_by()
    }

    fn buffer(&self) -> pictorus_traits::PassBy<'_, Self::Output> {
        self.output.as_by()
    }
}

impl<T: Float, const N: usize> HasIc for DerivativeBlock<T, N> {
    fn new(parameters: &Self::Parameters) -> Self {
        DerivativeBlock::<T, N> {
            samples: [T::zero(); N],
            sample_index: 0,
            initial_accumulation: true,
            output: parameters.ic,
        }
    }
}

impl<T: Float, const N: usize, const NCOLS: usize, const NROWS: usize> ProcessBlock
    for DerivativeBlock<Matrix<NROWS, NCOLS, T>, N>
{
    type Inputs = Matrix<NROWS, NCOLS, T>;
    type Output = Matrix<NROWS, NCOLS, T>;
    type Parameters = Parameters<Matrix<NROWS, NCOLS, T>>;

    fn process<'b>(
        &'b mut self,
        _parameters: &Self::Parameters,
        context: &dyn pictorus_traits::Context,
        inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) -> pictorus_traits::PassBy<'b, Self::Output> {
        // store the current input in the sample buffer
        self.samples[self.sample_index] = *inputs;

        // increment the sample index, wrapping at N (and setting initial_accumulation to false)
        self.sample_index += 1;
        if self.sample_index >= N {
            self.sample_index = 0;
            self.initial_accumulation = false;
        }

        // Only set the output when initial accumulation is done, otherwise use the IC
        if !self.initial_accumulation {
            let output = (inputs.as_view() - self.samples[self.sample_index].as_view())
                / ((T::from_usize(N).unwrap() - T::one())
                    * T::from_duration(context.timestep().expect(
                        "timestep should never be None outside of Initial Accumulation phase",
                    )));
            self.output.as_view_mut().copy_from(&output);
        }

        &self.output
    }

    fn buffer(&self) -> pictorus_traits::PassBy<'_, Self::Output> {
        self.output.as_by()
    }
}

impl<T: Float, const N: usize, const NCOLS: usize, const NROWS: usize> HasIc
    for DerivativeBlock<Matrix<NROWS, NCOLS, T>, N>
{
    fn new(parameters: &Self::Parameters) -> Self {
        DerivativeBlock::<Matrix<NROWS, NCOLS, T>, N> {
            samples: [Matrix::zeroed(); N],
            sample_index: 0,
            initial_accumulation: true,
            output: parameters.ic,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Parameters<T: Pass> {
    pub ic: T,
}

impl<T: Pass> Parameters<T> {
    pub fn new(ic: T) -> Self {
        Self { ic }
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use super::*;
    use crate::testing::StubRuntime;

    #[test]
    fn test_scalar() {
        let mut runtime = StubRuntime::default();
        runtime.context.fundamental_timestep = Duration::from_secs(1);
        let parameters = Parameters::new(0.0);
        let mut block = DerivativeBlock::<f64, 2>::new(&parameters);

        let input = 1.0;
        let output = block.process(&parameters, &runtime.context(), input);
        assert_eq!(output, 0.0);

        runtime.tick();
        let input = 2.0;
        let output = block.process(&parameters, &runtime.context(), input);
        assert_eq!(output, 1.0);

        runtime.tick();
        let input = 3.0;
        let output = block.process(&parameters, &runtime.context(), input);
        assert_eq!(output, 1.0);

        runtime.tick();
        let input = 4.0;
        let output = block.process(&parameters, &runtime.context(), input);
        assert_eq!(output, 1.0);
    }

    #[test]
    fn test_matrix() {
        let mut runtime = StubRuntime::default();
        runtime.context.fundamental_timestep = Duration::from_secs(1);
        let parameters = Parameters::new(Matrix::zeroed());
        let mut block = DerivativeBlock::<Matrix<2, 2, f32>, 2>::new(&parameters);

        let input = Matrix {
            data: [[1.0, 2.0], [3.0, 4.0]],
        };
        let output = block.process(&parameters, &runtime.context(), &input);
        assert_eq!(output, &Matrix::zeroed());

        runtime.tick();
        let input = Matrix {
            data: [[2.0, 3.0], [4.0, 5.0]],
        };
        let output = block.process(&parameters, &runtime.context(), &input);
        assert_eq!(
            output,
            &Matrix {
                data: [[1.0, 1.0], [1.0, 1.0]],
            }
        );

        runtime.tick();
        let input = Matrix {
            data: [[3.0, 4.0], [5.0, 6.0]],
        };
        let output = block.process(&parameters, &runtime.context(), &input);
        assert_eq!(
            output,
            &Matrix {
                data: [[1.0, 1.0], [1.0, 1.0]],
            }
        );
    }
}
