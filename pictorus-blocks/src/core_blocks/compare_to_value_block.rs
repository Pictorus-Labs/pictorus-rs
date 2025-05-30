use block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock, Scalar};

use super::comparison_block::ComparisonType;

pub struct Parameter<S: Scalar> {
    // The type of comparison to perform.
    pub comparison_type: ComparisonType,
    // The scalar value to compare against.
    pub value: S,
}

impl<S> Parameter<S>
where
    S: Scalar,
{
    pub fn new(comparison_type: &str, value: S) -> Self {
        Self {
            comparison_type: comparison_type
                .parse()
                .expect("Failed to parse comparison type"),
            value,
        }
    }
}

/// Compares the input to a scalar value.
///
/// The output is the same size as the input and each element is the result of the comparison.
pub struct CompareToValueBlock<T: Pass> {
    pub data: OldBlockData,
    buffer: Option<T>,
}

impl<T> Default for CompareToValueBlock<T>
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

macro_rules! impl_compare_to_value_block {
    ($type:ty) => {
        impl ProcessBlock for CompareToValueBlock<$type>
        where
            OldBlockData: FromPass<$type>,
        {
            type Inputs = $type;
            type Output = $type;
            type Parameters = Parameter<$type>;

            fn process(
                &mut self,
                parameters: &Self::Parameters,
                _context: &dyn pictorus_traits::Context,
                input: PassBy<Self::Inputs>,
            ) -> PassBy<Self::Output> {
                let val = match parameters.comparison_type {
                    ComparisonType::Equal => input == parameters.value,
                    ComparisonType::NotEqual => input != parameters.value,
                    ComparisonType::LessThan => input < parameters.value,
                    ComparisonType::LessOrEqual => input <= parameters.value,
                    ComparisonType::GreaterThan => input > parameters.value,
                    ComparisonType::GreaterOrEqual => input >= parameters.value,
                };
                let output = self.buffer.insert(val.into());
                self.data = OldBlockData::from_scalar((*output).into());
                *output
            }
        }

        impl<const ROWS: usize, const COLS: usize> ProcessBlock
            for CompareToValueBlock<Matrix<ROWS, COLS, $type>>
        where
            OldBlockData: FromPass<Matrix<ROWS, COLS, $type>>,
        {
            type Inputs = Matrix<ROWS, COLS, $type>;
            type Output = Matrix<ROWS, COLS, $type>;
            type Parameters = Parameter<$type>;

            fn process(
                &mut self,
                parameters: &Self::Parameters,
                _context: &dyn pictorus_traits::Context,
                input: PassBy<Self::Inputs>,
            ) -> PassBy<Self::Output> {
                let mut b = Matrix::<ROWS, COLS, $type>::zeroed();
                for r in 0..ROWS {
                    for c in 0..COLS {
                        let val = match parameters.comparison_type {
                            ComparisonType::Equal => input.data[c][r] == parameters.value,
                            ComparisonType::NotEqual => input.data[c][r] != parameters.value,
                            ComparisonType::LessThan => input.data[c][r] < parameters.value,
                            ComparisonType::LessOrEqual => input.data[c][r] <= parameters.value,
                            ComparisonType::GreaterThan => input.data[c][r] > parameters.value,
                            ComparisonType::GreaterOrEqual => input.data[c][r] >= parameters.value,
                        };
                        b.data[c][r] = val.into();
                    }
                }
                let output = self.buffer.insert(b);
                self.data = OldBlockData::from_pass(output);
                output
            }
        }
    };
}

impl_compare_to_value_block!(i8);
impl_compare_to_value_block!(u8);
impl_compare_to_value_block!(i16);
impl_compare_to_value_block!(u16);
impl_compare_to_value_block!(i32);
impl_compare_to_value_block!(u32);
impl_compare_to_value_block!(f32);
impl_compare_to_value_block!(f64);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use num_traits::{One, Zero};
    use paste::paste;

    macro_rules! test_compare_to_value {
        ($name:ident, $type:ty) => {
            paste! {
                #[test]
                fn [<test_compare_by_value_scalar_ $name>]() {
                    /*
                        Compares an input of 1 to a scalar value of 1 for all comparison types.
                    */
                    let mut parameters = Parameter::new("Equal", <$type>::one());
                    let context = StubContext::default();

                    let mut block = CompareToValueBlock::<$type>::default();

                    let output = block.process(&parameters, &context, <$type>::one());
                    assert_eq!(output, <$type>::one());
                    assert_eq!(block.data.scalar(), <$type>::one().into());

                    parameters.comparison_type = ComparisonType::NotEqual;
                    let output = block.process(&parameters, &context, <$type>::zero());
                    assert_eq!(output, <$type>::one());
                    assert_eq!(block.data.scalar(), <$type>::one().into());

                    parameters.comparison_type = ComparisonType::LessThan;
                    let output = block.process(&parameters, &context, <$type>::zero());
                    assert_eq!(output, <$type>::one());
                    assert_eq!(block.data.scalar(), <$type>::one().into());

                    parameters.comparison_type = ComparisonType::LessOrEqual;
                    let output = block.process(&parameters, &context, <$type>::one());
                    assert_eq!(output, <$type>::one());
                    assert_eq!(block.data.scalar(), <$type>::one().into());

                    parameters.comparison_type = ComparisonType::GreaterThan;
                    let output = block.process(&parameters, &context, <$type>::one() + <$type>::one());
                    assert_eq!(output, <$type>::one());
                    assert_eq!(block.data.scalar(), <$type>::one().into());

                    parameters.comparison_type = ComparisonType::GreaterOrEqual;
                    let output = block.process(&parameters, &context, <$type>::one());
                    assert_eq!(output, <$type>::one());
                    assert_eq!(block.data.scalar(), <$type>::one().into());
                }

                #[test]
                fn [<test_compare_by_value_matrix_ $name>]() {
                    /*
                        Compares an input [[1, 0], [0, 2]] to a scalar value of 1 for all comparison types.
                    */
                    let mut parameters = Parameter::new("Equal", <$type>::one());
                    let context = StubContext::default();

                    let mut block = CompareToValueBlock::<Matrix<2, 2, $type>>::default();
                    let input = Matrix {
                        data: [[<$type>::one(), <$type>::zero()], [<$type>::zero(), <$type>::one() + <$type>::one()]],
                    };

                    let output = block.process(&parameters, &context, &input);
                    assert_eq!(output.data[0][0], <$type>::one());
                    assert_eq!(output.data[0][1], <$type>::zero());
                    assert_eq!(output.data[1][0], <$type>::zero());
                    assert_eq!(output.data[1][1], <$type>::zero());
                    assert_eq!(
                        block.data,
                        OldBlockData::from_matrix(&[&[<$type>::one().into(), <$type>::zero().into()], &[<$type>::zero().into(), <$type>::zero().into()]])
                    );

                    parameters.comparison_type = ComparisonType::NotEqual;
                    let output = block.process(&parameters, &context, &input);
                    assert_eq!(output.data[0][0], <$type>::zero());
                    assert_eq!(output.data[0][1], <$type>::one());
                    assert_eq!(output.data[1][0], <$type>::one());
                    assert_eq!(output.data[1][1], <$type>::one());
                    assert_eq!(
                        block.data,
                        OldBlockData::from_matrix(&[&[<$type>::zero().into(), <$type>::one().into()], &[<$type>::one().into(), <$type>::one().into()]])
                    );

                    parameters.comparison_type = ComparisonType::LessThan;
                    let output = block.process(&parameters, &context, &input);
                    assert_eq!(output.data[0][0], <$type>::zero());
                    assert_eq!(output.data[0][1], <$type>::one());
                    assert_eq!(output.data[1][0], <$type>::one());
                    assert_eq!(output.data[1][1], <$type>::zero());
                    assert_eq!(
                        block.data,
                        OldBlockData::from_matrix(&[&[<$type>::zero().into(), <$type>::one().into()], &[<$type>::one().into(), <$type>::zero().into()]])
                    );

                    parameters.comparison_type = ComparisonType::LessOrEqual;
                    let output = block.process(&parameters, &context, &input);
                    assert_eq!(output.data[0][0], <$type>::one());
                    assert_eq!(output.data[0][1], <$type>::one());
                    assert_eq!(output.data[1][0], <$type>::one());
                    assert_eq!(output.data[1][1], <$type>::zero());
                    assert_eq!(
                        block.data,
                        OldBlockData::from_matrix(&[&[<$type>::one().into(), <$type>::one().into()], &[<$type>::one().into(), <$type>::zero().into()]])
                    );

                    parameters.comparison_type = ComparisonType::GreaterThan;
                    let output = block.process(&parameters, &context, &input);
                    assert_eq!(output.data[0][0], <$type>::zero());
                    assert_eq!(output.data[0][1], <$type>::zero());
                    assert_eq!(output.data[1][0], <$type>::zero());
                    assert_eq!(output.data[1][1], <$type>::one());
                    assert_eq!(
                        block.data,
                        OldBlockData::from_matrix(&[&[<$type>::zero().into(), <$type>::zero().into()], &[<$type>::zero().into(), <$type>::one().into()]])
                    );

                    parameters.comparison_type = ComparisonType::GreaterOrEqual;
                    let output = block.process(&parameters, &context, &input);
                    assert_eq!(output.data[0][0], <$type>::one());
                    assert_eq!(output.data[0][1], <$type>::zero());
                    assert_eq!(output.data[1][0], <$type>::zero());
                    assert_eq!(output.data[1][1], <$type>::one());
                    assert_eq!(
                        block.data,
                        OldBlockData::from_matrix(&[&[<$type>::one().into(), <$type>::zero().into()], &[<$type>::zero().into(), <$type>::one().into()]])
                    );
                }
            }
        };
    }

    test_compare_to_value!(i8, i8);
    test_compare_to_value!(u8, u8);
    test_compare_to_value!(i16, i16);
    test_compare_to_value!(u16, u16);
    test_compare_to_value!(i32, i32);
    test_compare_to_value!(u32, u32);
    test_compare_to_value!(f32, f32);
    test_compare_to_value!(f64, f64);
}
