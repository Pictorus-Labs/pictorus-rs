use crate::traits::Scalar;
use pictorus_traits::{Context, Matrix, Pass, PassBy, ProcessBlock};

/// Parameters for the Cast block
///
/// The destination type is carried in the block's generics rather than here, since it has
/// to be known at compile time.
#[doc(hidden)]
pub struct Parameters;

impl Default for Parameters {
    fn default() -> Self {
        Self::new()
    }
}

impl Parameters {
    pub fn new() -> Parameters {
        Parameters {}
    }
}

/// Converts a scalar element from one type to another.
///
/// This is deliberately not built on `num_traits::AsPrimitive` (as of 0.2.19), which has
/// incomplete bool conversion. Instead the three cases are handled explicitly:
/// - converting to `bool` tests truthiness
/// - converting from `bool` maps to 0/1
/// - numerics as a plain `as` cast.
pub trait CastElement<O: Scalar>: Scalar {
    fn cast_element(self) -> O;
}

// TODO: Add configurable truncation, etc. in the future.
/// Numeric -> numeric. Overflow, truncation and precision loss follow Rust's `as`
/// semantics.
macro_rules! impl_casts_from {
    ($from:ty => $($to:ty),* $(,)?) => {
        $(
            impl CastElement<$to> for $from {
                // Casting a type to itself is one of the pairs generated here.
                #[allow(clippy::unnecessary_cast)]
                #[inline]
                fn cast_element(self) -> $to {
                    self as $to
                }
            }
        )*
    };
}

/// Numeric -> bool: non-zero is true. `as` does not permit casting to bool at all.
macro_rules! impl_casts_to_bool {
    ($($from:ty),* $(,)?) => {
        $(
            impl CastElement<bool> for $from {
                #[inline]
                fn cast_element(self) -> bool {
                    Scalar::is_truthy(&self)
                }
            }
        )*
    };
}

/// bool -> numeric: false/true map to 0/1. `as` permits bool to integers but not to
/// floats, so go through from_bool uniformly.
macro_rules! impl_casts_from_bool {
    ($($to:ty),* $(,)?) => {
        $(
            impl CastElement<$to> for bool {
                #[inline]
                fn cast_element(self) -> $to {
                    <$to as Scalar>::from_bool(self)
                }
            }
        )*
    };
}

// Probably could consolidate this more, but seems to balance readability and
// macro mechanics.
impl_casts_from!(u8 => u8, i8, u16, i16, u32, i32, u64, i64, f32, f64);
impl_casts_from!(i8 => u8, i8, u16, i16, u32, i32, u64, i64, f32, f64);
impl_casts_from!(u16 => u8, i8, u16, i16, u32, i32, u64, i64, f32, f64);
impl_casts_from!(i16 => u8, i8, u16, i16, u32, i32, u64, i64, f32, f64);
impl_casts_from!(u32 => u8, i8, u16, i16, u32, i32, u64, i64, f32, f64);
impl_casts_from!(i32 => u8, i8, u16, i16, u32, i32, u64, i64, f32, f64);
impl_casts_from!(u64 => u8, i8, u16, i16, u32, i32, u64, i64, f32, f64);
impl_casts_from!(i64 => u8, i8, u16, i16, u32, i32, u64, i64, f32, f64);
impl_casts_from!(f32 => u8, i8, u16, i16, u32, i32, u64, i64, f32, f64);
impl_casts_from!(f64 => u8, i8, u16, i16, u32, i32, u64, i64, f32, f64);

impl_casts_to_bool!(u8, i8, u16, i16, u32, i32, u64, i64, f32, f64);
impl_casts_from_bool!(u8, i8, u16, i16, u32, i32, u64, i64, f32, f64);

impl CastElement<bool> for bool {
    #[inline]
    fn cast_element(self) -> bool {
        self
    }
}

/// A signal that can be converted to signal type `O`, preserving shape.
///
/// Implemented for scalars and matrices of the same dimensions.
pub trait CastTo<O: Pass + Default>: Pass + Default {
    fn cast<'a>(input: PassBy<'_, Self>, dest: &'a mut O) -> PassBy<'a, O>;
}

impl<I, O> CastTo<O> for I
where
    I: Scalar + CastElement<O>,
    O: Scalar,
{
    fn cast<'a>(input: PassBy<'_, Self>, dest: &'a mut O) -> PassBy<'a, O> {
        *dest = input.cast_element();
        *dest
    }
}

impl<const ROWS: usize, const COLS: usize, I, O> CastTo<Matrix<ROWS, COLS, O>>
    for Matrix<ROWS, COLS, I>
where
    I: Scalar + CastElement<O>,
    O: Scalar,
{
    fn cast<'a>(
        input: PassBy<'_, Self>,
        dest: &'a mut Matrix<ROWS, COLS, O>,
    ) -> PassBy<'a, Matrix<ROWS, COLS, O>> {
        for c in 0..COLS {
            for r in 0..ROWS {
                dest.data[c][r] = input.data[c][r].cast_element();
            }
        }
        dest
    }
}

/// Converts the input signal to a different data type.
pub struct CastBlock<I, O>
where
    I: CastTo<O>,
    O: Pass + Default,
{
    buffer: O,
    phantom: core::marker::PhantomData<I>,
}

impl<I, O> Default for CastBlock<I, O>
where
    I: CastTo<O>,
    O: Pass + Default,
{
    fn default() -> Self {
        Self {
            buffer: O::default(),
            phantom: core::marker::PhantomData,
        }
    }
}

impl<I, O> ProcessBlock for CastBlock<I, O>
where
    I: CastTo<O>,
    O: Pass + Default,
{
    type Parameters = Parameters;
    type Inputs = I;
    type Output = O;

    fn process<'b>(
        &'b mut self,
        _parameters: &Self::Parameters,
        _context: &dyn Context,
        input: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        I::cast(input, &mut self.buffer)
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        self.buffer.as_by()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;

    fn cast_scalar<I, O>(input: I) -> O
    where
        I: CastTo<O> + Scalar,
        O: Scalar,
    {
        let mut block = CastBlock::<I, O>::default();
        let context = StubContext::default();
        block.process(&Parameters::new(), &context, input)
    }

    #[test]
    fn test_default_buffer() {
        let block = CastBlock::<f64, u8>::default();
        assert_eq!(block.buffer(), 0u8);

        let block = CastBlock::<Matrix<2, 2, f64>, Matrix<2, 2, u8>>::default();
        assert_eq!(block.buffer(), &Matrix::<2, 2, u8>::zeroed());
    }

    #[test]
    fn test_widening_is_exact() {
        assert_eq!(cast_scalar::<u8, f64>(42), 42.0);
        assert_eq!(cast_scalar::<u8, u32>(42), 42u32);
        assert_eq!(cast_scalar::<i16, i64>(-300), -300i64);
        assert_eq!(cast_scalar::<f32, f64>(0.5), 0.5f64);
    }

    #[test]
    fn test_identity_cast() {
        assert_eq!(cast_scalar::<f64, f64>(1.25), 1.25);
        assert_eq!(cast_scalar::<u8, u8>(7), 7u8);
        assert!(cast_scalar::<bool, bool>(true));
        assert!(!cast_scalar::<bool, bool>(false));
    }

    #[test]
    fn test_float_to_int_truncates_toward_zero() {
        assert_eq!(cast_scalar::<f64, u8>(3.9), 3u8);
        assert_eq!(cast_scalar::<f64, i32>(-3.9), -3i32);
    }

    #[test]
    fn test_float_to_int_saturates() {
        // Rust's `as` saturates rather than wrapping or producing UB.
        assert_eq!(cast_scalar::<f64, u8>(300.0), 255u8);
        assert_eq!(cast_scalar::<f64, u8>(-1.0), 0u8);
        assert_eq!(cast_scalar::<f64, i8>(-500.0), -128i8);
    }

    #[test]
    fn test_int_to_int_wraps() {
        assert_eq!(cast_scalar::<u16, u8>(300), 44u8);
        assert_eq!(cast_scalar::<i32, u8>(-1), 255u8);
    }

    #[test]
    fn test_to_bool_is_truthiness() {
        assert!(!cast_scalar::<f64, bool>(0.0));
        assert!(cast_scalar::<f64, bool>(-0.1));
        assert!(!cast_scalar::<u8, bool>(0));
        assert!(cast_scalar::<u8, bool>(2));
    }

    #[test]
    fn test_from_bool_maps_to_zero_and_one() {
        assert_eq!(cast_scalar::<bool, f64>(true), 1.0);
        assert_eq!(cast_scalar::<bool, f64>(false), 0.0);
        assert_eq!(cast_scalar::<bool, u8>(true), 1u8);
        assert_eq!(cast_scalar::<bool, i64>(false), 0i64);
    }

    #[test]
    fn test_u64_above_2_53_loses_precision_in_f64() {
        // Documents the tradeoff that admitting u64/i64 to Scalar makes: the f64 interop
        // path is no longer exact for large magnitudes.
        let big = (1u64 << 53) + 1;
        assert_eq!(cast_scalar::<u64, f64>(big), 9007199254740992.0);
        assert_eq!(cast_scalar::<u64, u64>(big), big);
    }

    #[test]
    fn test_matrix_cast_is_element_wise() {
        let mut block = CastBlock::<Matrix<2, 2, f64>, Matrix<2, 2, u8>>::default();
        let context = StubContext::default();

        let input = Matrix::<2, 2, f64> {
            data: [[1.9, 2.1], [300.0, -1.0]],
        };
        let expected = Matrix::<2, 2, u8> {
            data: [[1, 2], [255, 0]],
        };

        assert_eq!(
            block.process(&Parameters::new(), &context, &input),
            &expected
        );
        assert_eq!(block.buffer(), &expected);
    }

    #[test]
    fn test_matrix_cast_to_bool() {
        let mut block = CastBlock::<Matrix<1, 3, f64>, Matrix<1, 3, bool>>::default();
        let context = StubContext::default();

        let input = Matrix::<1, 3, f64> {
            data: [[0.0], [1.0], [-2.5]],
        };
        let output = block.process(&Parameters::new(), &context, &input);

        assert_eq!(
            output,
            &Matrix::<1, 3, bool> {
                data: [[false], [true], [true]],
            }
        );
    }

    #[test]
    fn test_non_square_matrix_preserves_shape() {
        let mut block = CastBlock::<Matrix<3, 1, i32>, Matrix<3, 1, f32>>::default();
        let context = StubContext::default();

        let input = Matrix::<3, 1, i32> { data: [[-1, 0, 5]] };
        let output = block.process(&Parameters::new(), &context, &input);

        assert_eq!(
            output,
            &Matrix::<3, 1, f32> {
                data: [[-1.0, 0.0, 5.0]],
            }
        );
    }
}
