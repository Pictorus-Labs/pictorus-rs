use num_traits::NumCast;
use pictorus_block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock};

/// Shifts the bits of the input by a specified number of positions to the left or right.
///
/// For a matrix, the operation is applied component wise to each element.
pub struct BitShiftBlock<T>
where
    T: Apply,
{
    pub data: OldBlockData,
    buffer: Option<T::Output>,
}

impl<T> Default for BitShiftBlock<T>
where
    T: Apply,
    OldBlockData: FromPass<T::Output>,
{
    fn default() -> Self {
        Self {
            data: <OldBlockData as FromPass<T::Output>>::from_pass(T::Output::default().as_by()),
            buffer: None,
        }
    }
}

#[derive(strum::EnumString)]
pub enum ShiftDirection {
    Left,
    Right,
}

pub struct Parameters {
    // Direction of the bit shift: Left or Right
    pub direction: ShiftDirection,
    // Number of bits to shift by
    pub bits: u8,
}

impl Parameters {
    pub fn new(direction: &str, bits: impl NumCast) -> Self {
        Self {
            direction: direction.parse().expect("Failed to parse direction"),
            bits: bits.to_u8().expect("Failed to cast bits to u8"),
        }
    }
}

impl<T> ProcessBlock for BitShiftBlock<T>
where
    T: Apply,
    OldBlockData: FromPass<T::Output>,
{
    type Inputs = T;
    type Output = T::Output;
    type Parameters = Parameters;

    fn process(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        input: PassBy<Self::Inputs>,
    ) -> PassBy<'_, Self::Output> {
        let output = T::apply(&mut self.buffer, input, parameters);
        self.data = OldBlockData::from_pass(output);
        output
    }
}

pub trait Apply: Pass {
    type Output: Pass + Default;

    fn apply<'s>(
        store: &'s mut Option<Self::Output>,
        input: PassBy<Self>,
        params: &Parameters,
    ) -> PassBy<'s, Self::Output>;
}

macro_rules! impl_bit_shift_apply {
    ($type:ty, $cast_type:ty) => {
        impl Apply for $type {
            type Output = $type;

            fn apply<'s>(
                store: &'s mut Option<Self::Output>,
                input: PassBy<Self>,
                params: &Parameters,
            ) -> PassBy<'s, Self::Output> {
                let input = input as $cast_type;
                let output = match params.direction {
                    ShiftDirection::Left => input << params.bits,
                    ShiftDirection::Right => input >> params.bits,
                } as $type;
                *store = Some(output);
                output
            }
        }

        impl<const NROWS: usize, const NCOLS: usize> Apply for Matrix<NROWS, NCOLS, $type> {
            type Output = Matrix<NROWS, NCOLS, $type>;

            fn apply<'s>(
                store: &'s mut Option<Self::Output>,
                input: PassBy<Self>,
                params: &Parameters,
            ) -> PassBy<'s, Self::Output> {
                let output = store.insert(Matrix::zeroed());
                for i in 0..NROWS {
                    for j in 0..NCOLS {
                        let input_val = input.data[j][i] as $cast_type;
                        let res = match params.direction {
                            ShiftDirection::Left => input_val << params.bits,
                            ShiftDirection::Right => input_val >> params.bits,
                        } as $type;
                        output.data[j][i] = res;
                    }
                }
                output
            }
        }
    };
}

impl_bit_shift_apply!(i8, i8);
impl_bit_shift_apply!(i16, i16);
impl_bit_shift_apply!(i32, i32);
impl_bit_shift_apply!(f32, i32);
impl_bit_shift_apply!(f64, i64);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use paste::paste;

    macro_rules! test_bit_shift {
        ($type:ty) => {
            paste! {
                #[test]
                fn [<test_left_shift_scalar_ $type>]() {
                    let mut block = BitShiftBlock::<$type>::default();
                    let context = StubContext::default();
                    let params = Parameters::new("Left", 2);
                    let output = block.process(&params, &context, [<1 $type>]);
                    assert_eq!(output, [<4 $type>]);
                    assert_eq!(block.data.scalar(), 4.0);
                }

                #[test]
                fn [<test_right_shift_scalar_ $type>]() {
                    let mut block = BitShiftBlock::<$type>::default();
                    let context = StubContext::default();
                    let params = Parameters::new("Right", 2);
                    let output = block.process(&params, &context, [<8 $type>]);
                    assert_eq!(output, [<2 $type>]);
                    assert_eq!(block.data.scalar(), 2.0);

                    let output = block.process(&params, &context, [<2 $type>]);
                    assert_eq!(output, [<0 $type>]);
                    assert_eq!(block.data.scalar(), 0.0);
                }

                #[test]
                fn [<test_left_shift_matrix_ $type>]() {
                    let mut block = BitShiftBlock::<Matrix<2, 2, $type>>::default();
                    let context = StubContext::default();
                    let params = Parameters::new("Left", 2);
                    let input = Matrix {
                        data: [[[<1 $type>], [<2 $type>]], [[<3 $type>], [<4 $type>]]],
                    };
                    let output = block.process(&params, &context, &input);
                    assert_eq!(output.data, [[[<4 $type>], [<8 $type>]], [[<12 $type>], [<16 $type>]]]);
                    assert_eq!(
                        block.data.get_data().as_slice(),
                        [[4., 8.], [12., 16.]].as_flattened()
                    );
                }

                #[test]
                fn [<test_right_shift_matrix_ $type>]() {
                    let mut block = BitShiftBlock::<Matrix<2, 2, $type>>::default();
                    let context = StubContext::default();
                    let params = Parameters::new("Right", 2);
                    let input = Matrix {
                        data: [[[<4 $type>], [<8 $type>]], [[<12 $type>], [<16 $type>]]],
                    };
                    let output = block.process(&params, &context, &input);
                    assert_eq!(output.data, [[[<1 $type>], [<2 $type>]], [[<3 $type>], [<4 $type>]]]);
                    assert_eq!(
                        block.data.get_data().as_slice(),
                        [[1., 2.], [3., 4.]].as_flattened()
                    );
                }
            }
        };
    }

    test_bit_shift!(i8);
    test_bit_shift!(i16);
    test_bit_shift!(i32);
    test_bit_shift!(f32);
    test_bit_shift!(f64);
}
