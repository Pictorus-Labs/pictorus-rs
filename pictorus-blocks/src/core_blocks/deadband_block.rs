use num_traits::Zero;
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock};

use crate::{traits::MatrixOps, Scalar};

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
    buffer: T,
}

impl<T> Default for DeadbandBlock<T>
where
    T: Pass + Default,
{
    fn default() -> Self {
        Self {
            buffer: T::default(),
        }
    }
}

impl<S: Scalar + PartialOrd<S> + Default + Zero> ProcessBlock for DeadbandBlock<S> {
    type Inputs = S;
    type Output = S;
    type Parameters = Parameters<S>;

    fn process(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        input: PassBy<Self::Inputs>,
    ) -> PassBy<'_, Self::Output> {
        let in_deadband = input < parameters.upper_limit && input > parameters.lower_limit;
        let res = if in_deadband { S::zero() } else { input };
        self.buffer = res;
        self.buffer
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        self.buffer.as_by()
    }
}
impl<S: Scalar + PartialOrd<S> + Default + Zero, const NROWS: usize, const NCOLS: usize>
    ProcessBlock for DeadbandBlock<Matrix<NROWS, NCOLS, S>>
{
    type Inputs = Matrix<NROWS, NCOLS, S>;
    type Output = Matrix<NROWS, NCOLS, S>;
    type Parameters = Parameters<S>;

    fn process(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        input: PassBy<Self::Inputs>,
    ) -> PassBy<'_, Self::Output> {
        self.buffer = Matrix::zeroed();
        input.for_each(|v, c, r| {
            let in_deadband = v < parameters.upper_limit && v > parameters.lower_limit;
            self.buffer.data[c][r] = if in_deadband { S::zero() } else { v };
        });
        &self.buffer
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        self.buffer.as_by()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use paste::paste;

    #[test]
    fn test_deadband_default_buffer_no_panic() {
        let block = DeadbandBlock::<f64>::default();
        assert_eq!(block.buffer(), 0.0);

        let block = DeadbandBlock::<Matrix<2, 2, f64>>::default();
        assert_eq!(block.buffer(), &Matrix::<2, 2, f64>::zeroed());
    }

    macro_rules! test_deadband_block {
         // Convenience call to generate a call to the main macro for every type in the list
        ($type:ty, $($other_types:ty),*) => {
            test_deadband_block!($type);
            test_deadband_block!($($other_types),*);
        };
        ($type:ty) => {
            paste! {
                #[test]
                fn [<test_deadband_block_ $type>]() {
                    let (lower_limit,  upper_limit) = if (<$type>::MIN as f64).lt(&0.0) {
                        ((0 - 10) as $type,  10 as $type)
                    } else {
                        (10 as $type,  20 as $type)
                    };

                    const ZERO: $type = 0 as $type;
                    let mut block = DeadbandBlock::<$type>::default();
                    let parameters = Parameters::new(lower_limit, upper_limit);
                    let ctxt = StubContext::default();

                    // Anything exactly at the deadband limits maintains data
                    let input = lower_limit;
                    let output = block.process(&parameters, &ctxt, input);
                    assert_eq!(output, lower_limit);
                    assert_eq!(block.buffer(), output);

                    let input = upper_limit;
                    let output = block.process(&parameters, &ctxt, input);
                    assert_eq!(output, upper_limit);

                    // Anything between the deadband is set to zero.
                    let input = lower_limit + 1 as $type;
                    let output = block.process(&parameters, &ctxt, input);
                    assert_eq!(output, ZERO);

                    let input = ZERO;
                    let output = block.process(&parameters, &ctxt, input);
                    assert_eq!(output, ZERO);

                    let input = upper_limit - 1 as $type;
                    let output = block.process(&parameters, &ctxt, input);
                    assert_eq!(output, ZERO);
                }

                #[test]
                fn [<test_deadband_block_matrix_ $type>]() {
                    let (lower_limit,  upper_limit) = if (<$type>::MIN as f64).lt(&0.0) {
                        ((0-10) as $type,  10 as $type)
                    } else {
                        (10 as $type,  20 as $type)
                    };
                    const ZERO: $type = 0 as $type;
                    let mut block = DeadbandBlock::<Matrix<2, 2, $type>>::default();
                    let parameters = Parameters::new(lower_limit, upper_limit);
                    let ctxt = StubContext::default();

                    // Anything exactly at the deadband limits maintains data
                    let input = Matrix {
                        data: [[lower_limit, upper_limit], [upper_limit, lower_limit]],
                    };
                    let output = block.process(&parameters, &ctxt, &input);
                    assert_eq!(output.data, [[lower_limit, upper_limit], [upper_limit, lower_limit]]);

                    // Anything between the deadband is set to zero.
                    let input = Matrix {
                        data: [[lower_limit + 1 as $type, ZERO], [ZERO, upper_limit - 1 as $type]],
                    };
                    let output = block.process(&parameters, &ctxt, &input);
                    assert_eq!(output.data, [[ZERO, ZERO], [ZERO, ZERO]]);
                }
            }
        };
    }

    test_deadband_block!(f32, f64, i8, i16, i32, i64, u8, u16, u32, u64);
}
