use crate::traits::Scalar;
use num_traits::Signed;
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock};

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

impl<T: Scalar + Signed> ProcessBlock for AbsBlock<T> {
    type Inputs = T;
    type Output = T;
    type Parameters = Parameter;

    fn process<'b>(
        &'b mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        let output = Signed::abs(&inputs);
        self.buffer = output;
        output
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        self.buffer.as_by()
    }
}

impl<const ROWS: usize, const COLS: usize, T: Scalar + Signed> ProcessBlock
    for AbsBlock<Matrix<ROWS, COLS, T>>
{
    type Inputs = Matrix<ROWS, COLS, T>;
    type Output = Matrix<ROWS, COLS, T>;
    type Parameters = Parameter;

    fn process(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        input: PassBy<Self::Inputs>,
    ) -> PassBy<'_, Self::Output> {
        for (elem, &val) in self
            .buffer
            .data
            .as_flattened_mut()
            .iter_mut()
            .zip(input.data.as_flattened().iter())
        {
            *elem = val.abs();
        }
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
    use num_traits::One;
    use paste::paste;

    macro_rules! test_abs_block {
        ( $type:ty, $($other_types:ty),* ) => {
            test_abs_block!($type);
            $(
                test_abs_block!($other_types);
            )*
        };
        ( $type:ty) => {
            paste! {
                #[test]
                fn [<test_abs_block_default_buffer_ $type>]() {
                    let block = AbsBlock::<$type>::default();
                    assert_eq!(block.buffer(), <$type>::default());

                    let block = AbsBlock::<Matrix<2, 2, $type>>::default();
                    assert_eq!(block.buffer(), &Matrix::<2, 2, $type>::zeroed());
                }

                #[test]
                fn [<test_abs_block_scalar_ $type>]()
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
                fn [<test_abs_block_vector_1x2_ $type>]() {
                    let mut block = AbsBlock::<Matrix<1, 2, $type>>::default();
                    let context = StubContext::default();
                    let input = Matrix {
                        data: [[<$type>::one()], [-<$type>::one()]],
                    };
                    let expected = Matrix {
                        data: [[<$type>::one()], [<$type>::one()]],
                    };

                    let output = block.process(&Parameter::new(), &context, &input);
                    assert_eq!(output, &expected);
                    assert_eq!(block.buffer(), &expected);
                }

                #[test]
                fn [<test_abs_block_vector_2x1_ $type>]() {
                    let mut block = AbsBlock::<Matrix<2, 1, $type>>::default();
                    let context = StubContext::default();
                    let input = Matrix {
                        data: [[<$type>::one(), -<$type>::one()]],
                    };
                    let expected = Matrix {
                        data: [[<$type>::one(), <$type>::one()]],
                    };

                    let output = block.process(&Parameter::new(), &context, &input);
                    assert_eq!(output, &expected);
                    assert_eq!(block.buffer(), &expected);
                }

                #[test]
                fn [<test_abs_block_matrix_ $type>]() {
                    let mut block = AbsBlock::<Matrix<2, 2, $type>>::default();
                    let context = StubContext::default();
                    let input = Matrix {
                        data: [
                            [<$type>::one(), -<$type>::one()],
                            [<$type>::one(), -<$type>::one()],
                        ],
                    };
                    let expected = Matrix {
                        data: [
                            [<$type>::one(), <$type>::one()],
                            [<$type>::one(), <$type>::one()],
                        ],
                    };

                    let output = block.process(&Parameter::new(), &context, &input);
                    assert_eq!(output, &expected);
                    assert_eq!(block.buffer(), &expected);
                }
            }
        };
    }
    test_abs_block!(f32, f64, i8, i16, i32, i64);

    #[test]
    #[should_panic]
    fn overflow_quirk() {
        // According to num_traits docs this should return `::MIN` for signed integers, but instead it appears to panic:
        // https://docs.rs/num-traits/latest/num_traits/sign/trait.Signed.html#tymethod.abs
        let mut block = AbsBlock::<i8>::default();
        let context = StubContext::default();

        let output = block.process(&Parameter::new(), &context, i8::MIN);
        assert_eq!(output, -128i8);
    }
}
