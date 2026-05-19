use crate::matrix_ext::MatrixNalgebraExt;
use num_traits::Float;
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock, Scalar};

pub struct Parameter {}

impl Default for Parameter {
    fn default() -> Self {
        Self::new()
    }
}

impl Parameter {
    pub fn new() -> Self {
        Self {}
    }
}

/// Computes the absolute value of a scalar, vector, or matrix.
pub struct AbsBlock<T: Pass + Default> {
    buffer: T,
}

impl<T> Default for AbsBlock<T>
where
    T: Pass + Default,
{
    fn default() -> Self {
        Self {
            buffer: T::default(),
        }
    }
}

macro_rules! impl_abs_block {
    ($type:ty) => {
        impl ProcessBlock for AbsBlock<$type>
        where
            $type: Scalar,
        {
            type Inputs = $type;
            type Output = $type;
            type Parameters = Parameter;

            fn process<'b>(
                &'b mut self,
                _parameters: &Self::Parameters,
                _context: &dyn pictorus_traits::Context,
                inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
            ) -> pictorus_traits::PassBy<'b, Self::Output> {
                let output = Float::abs(inputs);
                self.buffer = output;
                output
            }

            fn buffer(&self) -> PassBy<'_, Self::Output> {
                self.buffer.as_by()
            }
        }

        impl<const ROWS: usize, const COLS: usize> ProcessBlock
            for AbsBlock<Matrix<ROWS, COLS, $type>>
        where
            $type: Scalar,
        {
            type Inputs = Matrix<ROWS, COLS, $type>;
            type Output = Matrix<ROWS, COLS, $type>;
            type Parameters = Parameter;

            fn process(
                &mut self,
                _parameters: &Self::Parameters,
                _context: &dyn pictorus_traits::Context,
                input: PassBy<Self::Inputs>,
            ) -> PassBy<'_, Self::Output> {
                let abs = input.as_view().abs();
                self.buffer = Matrix::<ROWS, COLS, $type>::from_view(&abs.as_view());
                &self.buffer
            }

            fn buffer(&self) -> PassBy<'_, Self::Output> {
                self.buffer.as_by()
            }
        }
    };
}

impl_abs_block!(f32);
impl_abs_block!(f64);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use num_traits::One;
    use paste::paste;

    macro_rules! test_abs_block {
        ($name:ident, $type:ty) => {
            paste! {
                #[test]
                fn [<test_abs_block_default_buffer_ $name>]() {
                    let block = AbsBlock::<$type>::default();
                    assert_eq!(block.buffer(), <$type>::default());

                    let block = AbsBlock::<Matrix<2, 2, $type>>::default();
                    assert_eq!(block.buffer(), &Matrix::<2, 2, $type>::zeroed());
                }

                #[test]
                fn [<test_abs_block_scalar_ $name>]()
                {
                    let mut block = AbsBlock::<$type>::default();
                    let context = StubContext::default();

                    let output = block.process(&Parameter::new(), &context, <$type>::one());
                    assert_eq!(output, <$type>::one());
                    assert_eq!(block.buffer(), output);

                    let output = block.process(&Parameter::new(), &context, -<$type>::one());
                    assert_eq!(output, <$type>::one());
                    assert_eq!(block.buffer(), <$type>::one());
                }

                #[test]
                fn [<test_abs_block_vector_1x2_ $name>]() {
                    let mut block = AbsBlock::<Matrix<1, 2, $type>>::default();
                    let context = StubContext::default();
                    let mut input = Matrix::<1, 2, $type>::zeroed();
                    input.data[0][0] = <$type>::one();
                    input.data[1][0] = -<$type>::one();

                    let output = block.process(&Parameter::new(), &context, &input);
                    assert_eq!(output.data[0][0], <$type>::one());
                    assert_eq!(output.data[1][0], <$type>::one());
                    assert_eq!(block.buffer().data[0][0], <$type>::one());
                    assert_eq!(block.buffer().data[1][0], <$type>::one());
                }

                #[test]
                fn [<test_abs_block_vector_2x1_ $name>]() {
                    let mut block = AbsBlock::<Matrix<2, 1, $type>>::default();
                    let context = StubContext::default();
                    let mut input = Matrix::<2, 1, $type>::zeroed();
                    input.data[0][0] = <$type>::one();
                    input.data[0][1] = -<$type>::one();

                    let output = block.process(&Parameter::new(), &context, &input);
                    assert_eq!(output.data[0][0], <$type>::one());
                    assert_eq!(output.data[0][1], <$type>::one());
                    assert_eq!(block.buffer().data[0][0], <$type>::one());
                    assert_eq!(block.buffer().data[0][1], <$type>::one());
                }

                #[test]
                fn [<test_abs_block_matrix_ $name>]() {
                    let mut block = AbsBlock::<Matrix<2, 2, $type>>::default();
                    let context = StubContext::default();
                    let mut input = Matrix::<2, 2, $type>::zeroed();
                    input.data[0][0] = <$type>::one();
                    input.data[0][1] = -<$type>::one();
                    input.data[1][0] = <$type>::one();
                    input.data[1][1] = -<$type>::one();

                    let output = block.process(&Parameter::new(), &context, &input);
                    assert_eq!(output.data[0][0], <$type>::one());
                    assert_eq!(output.data[0][1], <$type>::one());
                    assert_eq!(output.data[1][0], <$type>::one());
                    assert_eq!(output.data[1][1], <$type>::one());
                    assert_eq!(block.buffer().data[0][0], <$type>::one());
                    assert_eq!(block.buffer().data[0][1], <$type>::one());
                    assert_eq!(block.buffer().data[1][0], <$type>::one());
                    assert_eq!(block.buffer().data[1][1], <$type>::one());
                }
            }
        };
    }

    test_abs_block!(f32, f32);
    test_abs_block!(f64, f64);
}
