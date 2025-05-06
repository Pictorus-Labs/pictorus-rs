use crate::nalgebra_interop::MatrixExt;
use block_data::{BlockData as OldBlockData, FromPass};
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
    pub data: OldBlockData,
    buffer: Option<T>,
}

impl<T> Default for AbsBlock<T>
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

macro_rules! impl_abs_block {
    ($type:ty) => {
        impl ProcessBlock for AbsBlock<$type>
        where
            $type: Scalar,
            OldBlockData: FromPass<$type>,
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
                self.data = OldBlockData::from_scalar(output.into());
                output
            }
        }

        impl<const ROWS: usize, const COLS: usize> ProcessBlock
            for AbsBlock<Matrix<ROWS, COLS, $type>>
        where
            $type: Scalar,
            OldBlockData: FromPass<Matrix<ROWS, COLS, $type>>,
        {
            type Inputs = Matrix<ROWS, COLS, $type>;
            type Output = Matrix<ROWS, COLS, $type>;
            type Parameters = Parameter;

            fn process(
                &mut self,
                _parameters: &Self::Parameters,
                _context: &dyn pictorus_traits::Context,
                input: PassBy<Self::Inputs>,
            ) -> PassBy<Self::Output> {
                let abs = input.as_view().abs();
                let o = Matrix::<ROWS, COLS, $type>::from_view(&abs.as_view());
                let output = self.buffer.insert(o);
                self.data = OldBlockData::from_pass(output);
                output
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
                fn [<test_abs_block_scalar_ $name>]()
                {
                    let mut block = AbsBlock::<$type>::default();
                    let context = StubContext::default();

                    let output = block.process(&Parameter::new(), &context, <$type>::one());
                    assert_eq!(output, <$type>::one());
                    assert_eq!(block.data, OldBlockData::from_scalar(<$type>::one().into()));

                    let output = block.process(&Parameter::new(), &context, -<$type>::one());
                    assert_eq!(output, <$type>::one());
                    assert_eq!(block.data, OldBlockData::from_scalar(1.0));
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
                    assert_eq!(block.data, OldBlockData::from_matrix(&[&[<$type>::one().into(), <$type>::one().into()]]));
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
                    assert_eq!(block.data, OldBlockData::from_matrix(&[&[<$type>::one().into()], &[<$type>::one().into()]]));
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
                    assert_eq!(block.data, OldBlockData::from_matrix(&[&[<$type>::one().into(), <$type>::one().into()], &[<$type>::one().into(), <$type>::one().into()]]));
                }
            }
        }
    }

    test_abs_block!(f32, f32);
    test_abs_block!(f64, f64);
}
