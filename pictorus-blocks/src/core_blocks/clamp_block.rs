use block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock};

pub struct Parameters<T> {
    /// The minimum value to clamp the input to.
    pub min: T,
    /// The maximum value to clamp the input to.
    pub max: T,
}

impl<T> Parameters<T> {
    pub fn new(min: T, max: T) -> Self {
        Self { min, max }
    }
}

/// Clamps an input based on the min and max values provided.
///
/// If an input is larger than the max value, it will be set to the max value. If
/// the input is less than the min value, it will be set to the min value.
pub struct ClampBlock<T> {
    pub data: OldBlockData,
    buffer: Option<T>,
}

impl<T> Default for ClampBlock<T>
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

macro_rules! impl_clamp_block {
    ($type:ty) => {
        impl ProcessBlock for ClampBlock<$type>
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
                let clamp = input.clamp(parameters.min, parameters.max);
                let output = self.buffer.insert(clamp);
                self.data = OldBlockData::from_scalar((*output).into());
                *output
            }
        }

        impl<const ROWS: usize, const COLS: usize> ProcessBlock
            for ClampBlock<Matrix<ROWS, COLS, $type>>
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
                for r in 0..ROWS {
                    for c in 0..COLS {
                        output.data[c][r] = input.data[c][r].clamp(parameters.min, parameters.max);
                    }
                }
                self.data = OldBlockData::from_pass(output);
                output
            }
        }
    };
}

impl_clamp_block!(i8);
impl_clamp_block!(u8);
impl_clamp_block!(i16);
impl_clamp_block!(u16);
impl_clamp_block!(i32);
impl_clamp_block!(u32);
impl_clamp_block!(f32);
impl_clamp_block!(f64);

#[cfg(test)]
mod test {
    use crate::testing::StubContext;
    use block_data::ToPass;
    use num_traits::{One, Zero};
    use paste::paste;

    use super::*;

    #[test]
    fn test_clamp_block_original() {
        let c = StubContext::default();
        let lower_limit: f64 = -1.5;
        let upper_limit: f64 = -0.5;
        let input = &OldBlockData::from_vector(&[1.0, -0.5, -1.2345, -1.6]);
        let mut block = ClampBlock::<Matrix<1, 4, f64>>::default();
        let p = Parameters::new(lower_limit, upper_limit);

        let output = block.process(&p, &c, &input.to_pass());
        assert_eq!(
            output,
            &Matrix {
                data: [[-0.5], [-0.5], [-1.2345], [-1.5]]
            }
        );
        assert_eq!(
            block.data,
            OldBlockData::from_matrix(&[&[-0.5, -0.5, -1.2345, -1.5]])
        )
    }

    macro_rules! impl_clamp_block_test_negatives {
        ($type:ty, $name:ident) => {
            paste! {
                #[test]
                fn [<test_clamp_block_scalar_negative_ $name>]() {
                    /* Clamp -1 to 1, test -2, 2, 0 */
                    let c = StubContext::default();
                    let mut block = ClampBlock::<$type>::default();
                    let parameters = Parameters::new(-$type::one(), $type::one());

                    let input = -($type::one() + $type::one());
                    let output = block.process(&parameters, &c, input.as_by());
                    assert_eq!(output, -$type::one());

                    let input = $type::one() + $type::one();
                    let output = block.process(&parameters, &c, input.as_by());
                    assert_eq!(output, $type::one());

                    let input = $type::zero();
                    let output = block.process(&parameters, &c, input.as_by());
                    assert_eq!(output, $type::zero());
                }

                #[test]
                fn [<test_clamp_block_matrix_negative_ $name>]() {
                    /* Clamp -1, 1, test [[2, 0], [1, -2]] */
                    let c = StubContext::default();
                    let mut block = ClampBlock::<Matrix<2, 2, $type>>::default();
                    let parameters = Parameters::new(-$type::one(), $type::one());

                    let neg_2 = -($type::one() + $type::one());
                    let pos_2 = $type::one() + $type::one();

                    let input = Matrix {
                        data: [[pos_2.into(), $type::zero().into()], [$type::one().into(), neg_2.into()]]
                    };
                    let output = block.process(&parameters, &c, &input);
                    assert_eq!(
                        output,
                        &Matrix {
                            data: [[$type::one(), $type::zero()], [$type::one(), -$type::one()]]
                        }
                    );
                    assert_eq!(
                        block.data,
                        OldBlockData::from_matrix(&[&[$type::one().into(), $type::one().into()], &[$type::zero().into(), (-$type::one()).into()]])
                    );
                }
            }
        };
    }

    impl_clamp_block_test_negatives!(f64, f64);
    impl_clamp_block_test_negatives!(f32, f32);
    impl_clamp_block_test_negatives!(i32, i32);
    impl_clamp_block_test_negatives!(i16, i16);
    impl_clamp_block_test_negatives!(i8, i8);

    macro_rules! impl_clamp_block_test_positives {
        ($type:ty, $name:ident) => {
            paste! {
                #[test]
                fn [<test_clamp_block_scalar_positive_ $name>]() {
                    /* Clamp 1 to 2, test 0, 2, 3 */
                    let c = StubContext::default();
                    let mut block = ClampBlock::<$type>::default();

                    let pos_2 = $type::one() + $type::one();
                    let pos_3 = pos_2 + $type::one();

                    let parameters = Parameters::new($type::one(), pos_2);

                    let input = pos_3;
                    let output = block.process(&parameters, &c, input.as_by());
                    assert_eq!(output, pos_2);

                    let input = pos_2;
                    let output = block.process(&parameters, &c, input.as_by());
                    assert_eq!(output, pos_2);

                    let input = $type::zero();
                    let output = block.process(&parameters, &c, input.as_by());
                    assert_eq!(output, $type::one());
                }

                #[test]
                fn [<test_clamp_block_matrix_positive_ $name>]() {
                    /* Clamp 1 to 2, test [[3, 0], [1, 2]] */
                    let c = StubContext::default();
                    let mut block = ClampBlock::<Matrix<2, 2, $type>>::default();

                    let pos_2 = $type::one() + $type::one();
                    let pos_3 = pos_2 + $type::one();

                    let parameters = Parameters::new($type::one(), pos_2);

                    let input = Matrix {
                        data: [[pos_3.into(), $type::zero().into()], [$type::one().into(), pos_2.into()]]
                    };
                    let output = block.process(&parameters, &c, &input);
                    assert_eq!(
                        output,
                        &Matrix {
                            data: [[pos_2, $type::one()], [$type::one(), pos_2]]
                        }
                    );
                    assert_eq!(
                        block.data,
                        OldBlockData::from_matrix(&[&[pos_2.into(), $type::one().into()], &[$type::one().into(), pos_2.into()]])
                    );
                }
            }
        };
    }

    impl_clamp_block_test_positives!(f64, f64);
    impl_clamp_block_test_positives!(f32, f32);
    impl_clamp_block_test_positives!(u32, u32);
    impl_clamp_block_test_positives!(i32, i32);
    impl_clamp_block_test_positives!(i16, i16);
    impl_clamp_block_test_positives!(u16, u16);
    impl_clamp_block_test_positives!(i8, i8);
    impl_clamp_block_test_positives!(u8, u8);
}
