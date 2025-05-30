use crate::nalgebra_interop::MatrixExt;
use num_traits::One;
use paste::paste;
use pictorus_block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{HasIc, Matrix, Pass, ProcessBlock};

/// Compute the discrete derivative of a signal using a sliding window of samples.
pub struct DerivativeBlock<T: Pass + Default + Copy, const N: usize>
where
    pictorus_block_data::BlockData: FromPass<T>,
{
    samples: [T; N],
    sample_index: usize,
    initial_accumulation: bool,
    output: T,
    pub data: OldBlockData,
}

impl<const N: usize, T: Pass + Default + Copy> Default for DerivativeBlock<T, N>
where
    pictorus_block_data::BlockData: FromPass<T>,
{
    fn default() -> Self {
        Self {
            samples: [T::default(); N],
            sample_index: 0,
            initial_accumulation: true,
            output: T::default(),
            data: <OldBlockData as FromPass<T>>::from_pass(T::default().as_by()),
        }
    }
}

macro_rules! impl_process {
    ($type:ty) => {
        paste! {
        impl<const N: usize> ProcessBlock for DerivativeBlock< $type, N>
        {
            type Inputs = $type;
            type Output = $type;
            type Parameters = Parameters<$type>;

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
                        / ((N as $type - $type::one()) * context.timestep().expect("timestep should never be None outside of Initial Accumulation phase").[<as_secs_ $type>]());
                }

                self.data = <OldBlockData as FromPass<$type>>::from_pass(self.output);
                self.output.as_by()
            }
        }

        impl<const N: usize> HasIc for DerivativeBlock<$type, N>
        {
            fn new(parameters: &Self::Parameters) -> Self {
                DerivativeBlock::<$type, N> {
                    samples: [0.0; N],
                    sample_index: 0,
                    initial_accumulation: true,
                    output: parameters.ic,
                    data: <OldBlockData as FromPass<$type>>::from_pass(parameters.ic),
                }
            }
        }


        impl<const N: usize, const NCOLS: usize, const NROWS: usize> ProcessBlock for DerivativeBlock< Matrix<NROWS, NCOLS, $type>, N>
        {
            type Inputs = Matrix<NROWS, NCOLS, $type>;
            type Output = Matrix<NROWS, NCOLS, $type>;
            type Parameters = Parameters<Matrix<NROWS, NCOLS, $type>>;

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
                    let output =
                     (inputs.as_view() - self.samples[self.sample_index].as_view())
                        / ((N as $type - $type::one()) * context.timestep().expect("timestep should never be None outside of Initial Accumulation phase").[<as_secs_ $type>]());
                    self.output.as_view_mut().copy_from(&output);
                }

                self.data = <OldBlockData as FromPass<Matrix<NROWS, NCOLS, $type>>>::from_pass(self.output.as_by());
                &self.output
            }
        }

        impl<const N: usize, const NCOLS: usize, const NROWS: usize> HasIc for DerivativeBlock<Matrix<NROWS, NCOLS, $type>, N>
        {
            fn new(parameters: &Self::Parameters) -> Self {
                DerivativeBlock::<Matrix<NROWS, NCOLS, $type>, N> {
                    samples: [Matrix::zeroed(); N],
                    sample_index: 0,
                    initial_accumulation: true,
                    output: parameters.ic,
                    data: <OldBlockData as FromPass<Matrix<NROWS, NCOLS, $type>>>::from_pass(&parameters.ic),
                }
            }
        }



    }
    };
}

impl_process!(f64);
impl_process!(f32);

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
        let mut block = DerivativeBlock::<f64, 2>::default();
        let mut runtime = StubRuntime::default();
        runtime.context.fundamental_timestep = Duration::from_secs(1);
        let parameters = Parameters::new(0.0);

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
        let mut block = DerivativeBlock::<Matrix<2, 2, f32>, 2>::default();
        let mut runtime = StubRuntime::default();
        runtime.context.fundamental_timestep = Duration::from_secs(1);
        let parameters = Parameters::new(Matrix::zeroed());

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
