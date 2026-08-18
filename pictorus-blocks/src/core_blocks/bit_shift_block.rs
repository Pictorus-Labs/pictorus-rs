use num_traits::NumCast;
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock, Scalar};

/// Shifts the bits of the input by a specified number of positions to the left or right.
///
/// For a matrix, the operation is applied component wise to each element.
pub struct BitShiftBlock<T>
where
    T: Apply,
{
    buffer: T,
}

impl<T> Default for BitShiftBlock<T>
where
    T: Apply,
{
    fn default() -> Self {
        Self {
            buffer: T::default(),
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
{
    type Inputs = T;
    type Output = T;
    type Parameters = Parameters;

    fn process(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        input: PassBy<Self::Inputs>,
    ) -> PassBy<'_, Self::Output> {
        let output = T::apply(&mut self.buffer, input, parameters);
        output
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        self.buffer.as_by()
    }
}

pub trait Apply: Pass + Default {
    fn apply<'s>(store: &'s mut Self, input: PassBy<Self>, params: &Parameters)
        -> PassBy<'s, Self>;
}

macro_rules! impl_bit_shift_apply {
    ($type:ty) => {
        impl Apply for $type {
            fn apply<'s>(
                store: &'s mut Self,
                input: PassBy<Self>,
                params: &Parameters,
            ) -> PassBy<'s, Self> {
                let output = match params.direction {
                    ShiftDirection::Left => input << params.bits,
                    ShiftDirection::Right => input >> params.bits,
                };
                *store = output;
                output
            }
        }
    };
    ($type:ty, $cast_type:ty) => {
        impl Apply for $type {
            fn apply<'s>(
                store: &'s mut Self,
                input: PassBy<Self>,
                params: &Parameters,
            ) -> PassBy<'s, Self> {
                let input = input as $cast_type;
                let output = match params.direction {
                    ShiftDirection::Left => input << params.bits,
                    ShiftDirection::Right => input >> params.bits,
                } as $type;
                *store = output;
                output
            }
        }
    };
}

impl<const NROWS: usize, const NCOLS: usize, T: Scalar> Apply for Matrix<NROWS, NCOLS, T>
where
    T: Apply,
{
    fn apply<'s>(
        store: &'s mut Self,
        input: PassBy<Self>,
        params: &Parameters,
    ) -> PassBy<'s, Self> {
        for i in 0..NROWS {
            for j in 0..NCOLS {
                let input_val = input.data[j][i];
                T::apply(&mut store.data[j][i], input_val.as_by(), params);
            }
        }
        store
    }
}

impl_bit_shift_apply!(f32, i32);
impl_bit_shift_apply!(f64, i64);
impl_bit_shift_apply!(i8);
impl_bit_shift_apply!(i16);
impl_bit_shift_apply!(i32);
impl_bit_shift_apply!(i64);
impl_bit_shift_apply!(u8);
impl_bit_shift_apply!(u16);
impl_bit_shift_apply!(u32);
impl_bit_shift_apply!(u64);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use paste::paste;

    macro_rules! test_bit_shift {
        // Convenience call to generate a call to the main macro for every type in the list
        ($type:ty, $($other_types:ty),*) => {
            test_bit_shift!($type);
            test_bit_shift!($($other_types),*);
        };
        ($type:ty) => {
            paste! {
                #[test]
                fn [<test_default_buffer_no_panic_ $type>]() {
                    let block = BitShiftBlock::<$type>::default();
                    assert_eq!(block.buffer(), <$type>::default());
                    let block = BitShiftBlock::<Matrix<2, 2, $type>>::default();
                    assert_eq!(block.buffer(), &Matrix::<2, 2, $type>::zeroed());
                }

                #[test]
                fn [<test_left_shift_scalar_ $type>]() {
                    let mut block = BitShiftBlock::<$type>::default();
                    let context = StubContext::default();
                    let params = Parameters::new("Left", 2);
                    let output = block.process(&params, &context, [<1 $type>]);
                    assert_eq!(output, [<4 $type>]);
                    assert_eq!(block.buffer(), output);
                }

                #[test]
                fn [<test_right_shift_scalar_ $type>]() {
                    let mut block = BitShiftBlock::<$type>::default();
                    let context = StubContext::default();
                    let params = Parameters::new("Right", 2);
                    let output = block.process(&params, &context, [<8 $type>]);
                    assert_eq!(output, [<2 $type>]);
                    assert_eq!(block.buffer(), [<2 $type>]);

                    let output = block.process(&params, &context, [<2 $type>]);
                    assert_eq!(output, [<0 $type>]);
                    assert_eq!(block.buffer(), [<0 $type>]);
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
                    assert_eq!(block.buffer().data, [[[<4 $type>], [<8 $type>]], [[<12 $type>], [<16 $type>]]]);
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
                    assert_eq!(block.buffer().data, [[[<1 $type>], [<2 $type>]], [[<3 $type>], [<4 $type>]]]);
                }
            }
        };
    }

    test_bit_shift!(f64, f32, i8, i16, i32, i64, u8, u16, u32, u64);

    #[test]
    #[should_panic]
    fn shift_amount_exceeds_width_panics() {
        // Shifting by an amount >= the type's bit width panics in debug builds
        // (the shift amount wraps in release).
        let mut block = BitShiftBlock::<u8>::default();
        let context = StubContext::default();
        let params = Parameters::new("Left", 8);
        let output = block.process(&params, &context, 1u8);
        assert_eq!(output, 0);
    }

    #[test]
    #[should_panic]
    fn float_shift_amount_exceeds_cast_width_panics() {
        // Floats shift through a cast to i32 (f32) / i64 (f64), so the panic threshold
        // is the *cast* integer type's bit width — 32 for f32 — not anything about the
        // float itself. Panics in debug builds, wraps in release.
        let mut block = BitShiftBlock::<f32>::default();
        let context = StubContext::default();
        let params = Parameters::new("Left", 32);
        let output = block.process(&params, &context, 1.0f32);
        assert_eq!(output, 0.0);
    }
}
