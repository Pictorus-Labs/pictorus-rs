use block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock};

use crate::traits::MatrixOps;

pub struct Parameters<T> {
    // Lower limit of the deadband
    pub lower_limit: T,
    // Upper limit of the deadband
    pub upper_limit: T,
}

impl<T> Parameters<T> {
    pub fn new(lower_limit: T, upper_limit: T) -> Self {
        Self {
            lower_limit,
            upper_limit,
        }
    }
}

/// Implements a deadband on the input signal.
///
/// If the input is within the deadband, the output is zero. Otherwise, the input value is passed through.
pub struct DeadbandBlock<T> {
    pub data: OldBlockData,
    buffer: Option<T>,
}

impl<T> Default for DeadbandBlock<T>
where
    T: Pass + Default,
    OldBlockData: FromPass<T>,
{
    fn default() -> Self {
        Self {
            data: <OldBlockData as FromPass<T>>::from_pass(T::default().as_by()),
            buffer: None,
        }
    }
}

macro_rules! impl_deadband_block {
    ($type:ty) => {
        impl ProcessBlock for DeadbandBlock<$type>
        where
            OldBlockData: FromPass<$type>,
        {
            type Inputs = $type;
            type Output = $type;
            type Parameters = Parameters<$type>;

            fn process(
                &mut self,
                parameters: &Self::Parameters,
                _context: &dyn pictorus_traits::Context,
                input: PassBy<Self::Inputs>,
            ) -> PassBy<Self::Output> {
                let in_deadband = input < parameters.upper_limit && input > parameters.lower_limit;
                let res = if in_deadband { 0.0 } else { input };
                let output = self.buffer.insert(res);
                self.data = OldBlockData::from_scalar(res.into());
                *output
            }
        }

        impl<const ROWS: usize, const COLS: usize> ProcessBlock
            for DeadbandBlock<Matrix<ROWS, COLS, $type>>
        where
            OldBlockData: FromPass<Matrix<ROWS, COLS, $type>>,
        {
            type Inputs = Matrix<ROWS, COLS, $type>;
            type Output = Matrix<ROWS, COLS, $type>;
            type Parameters = Parameters<$type>;

            fn process(
                &mut self,
                parameters: &Self::Parameters,
                _context: &dyn pictorus_traits::Context,
                input: PassBy<Self::Inputs>,
            ) -> PassBy<Self::Output> {
                let output = self.buffer.insert(Matrix::zeroed());
                input.for_each(|v, c, r| {
                    let in_deadband = v < parameters.upper_limit && v > parameters.lower_limit;
                    output.data[c][r] = if in_deadband { 0.0 } else { v };
                });
                self.data = OldBlockData::from_pass(output);
                output
            }
        }
    };
}

impl_deadband_block!(f32);
impl_deadband_block!(f64);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use paste::paste;

    macro_rules! test_deadband_block {
        ($type:ty) => {
            paste! {
                #[test]
                fn [<test_deadband_block_ $type>]() {
                    let mut block = DeadbandBlock::<$type>::default();
                    let parameters = Parameters::new(-1.0, 1.0);
                    let ctxt = StubContext::default();

                    // Anything exactly at the deadband limits maintains data
                    let input = -1.0;
                    let output = block.process(&parameters, &ctxt, input);
                    assert_eq!(output, -1.0);

                    let input = 1.0;
                    let output = block.process(&parameters, &ctxt, input);
                    assert_eq!(output, 1.0);

                    // Anything between the deadband is set to zero.
                    let input = -0.999;
                    let output = block.process(&parameters, &ctxt, input);
                    assert_eq!(output, 0.0);

                    let input = 0.0;
                    let output = block.process(&parameters, &ctxt, input);
                    assert_eq!(output, 0.0);

                    let input = 0.999;
                    let output = block.process(&parameters, &ctxt, input);
                    assert_eq!(output, 0.0);
                }

                #[test]
                fn [<test_deadband_block_matrix_ $type>]() {
                    let mut block = DeadbandBlock::<Matrix<2, 2, $type>>::default();
                    let parameters = Parameters::new(-1.0, 1.0);
                    let ctxt = StubContext::default();

                    // Anything exactly at the deadband limits maintains data
                    let input = Matrix {
                        data: [[-1.0, 1.0], [1.0, -1.0]],
                    };
                    let output = block.process(&parameters, &ctxt, &input);
                    assert_eq!(output.data, [[-1.0, 1.0], [1.0, -1.0]]);

                    // Anything between the deadband is set to zero.
                    let input = Matrix {
                        data: [[-0.999, 0.0], [0.0, 0.999]],
                    };
                    let output = block.process(&parameters, &ctxt, &input);
                    assert_eq!(output.data, [[0.0, 0.0], [0.0, 0.0]]);
                }
            }
        };
    }

    test_deadband_block!(f32);
    test_deadband_block!(f64);
}
