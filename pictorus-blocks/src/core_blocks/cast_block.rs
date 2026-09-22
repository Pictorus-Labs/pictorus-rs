use crate::traits::Scalar;
use num_traits::{AsPrimitive, Float, PrimInt};
use pictorus_traits::{ModelClock, Matrix, Pass, PassBy, ProcessBlock};

/// Parameters for the Cast block
///
/// The destination type is carried in the block's generics rather than here, since it has
/// to be known at compile time.
#[doc(hidden)]
#[derive(Default)]
pub struct Parameters;

impl Parameters {
    pub fn new() -> Parameters {
        Self
    }
}

/// What a cast does when the value does not fit the destination type.
///
/// The two methods are generic over the type pair, so each mode is a single implementation
/// rather than one per pair.
pub trait OverflowMode {
    /// Convert a float to an integer, deciding what to do when it does not fit.
    ///
    /// Callers must apply the rounding mode first; `CastElement` does. A value that still
    /// has a fractional part is truncated toward zero by the conversion itself, which
    /// would silently bypass the chosen rounding mode.
    fn from_float<F: Float, T: PrimInt + 'static>(v: F) -> T
    where
        i128: AsPrimitive<T>;

    /// Convert between integer types, possibly narrowing.
    fn narrow_int<F: PrimInt + AsPrimitive<T>, T: PrimInt + 'static>(v: F) -> T;
}

/// Clamp out-of-range values to the destination's bounds.
pub struct Saturate;

/// Wrap out-of-range values modulo the destination's range.
///
/// A float is rounded and then wrapped like an integer would be, so `256.0 -> u8` is 0.
/// Values with no usable low bits fall back to clamping: NaN becomes zero and the
/// infinities saturate.
pub struct Wrap;

/// Panic when a value does not fit the destination type.
pub struct Panic;

impl OverflowMode for Saturate {
    #[inline]
    fn from_float<F: Float, T: PrimInt + 'static>(v: F) -> T
    where
        i128: AsPrimitive<T>,
    {
        if v.is_nan() {
            // `as` maps NaN to zero; there is no bound to clamp toward.
            return T::zero();
        }
        T::from(v).unwrap_or(if v > F::zero() {
            T::max_value()
        } else {
            T::min_value()
        })
    }

    #[inline]
    fn narrow_int<F: PrimInt + AsPrimitive<T>, T: PrimInt + 'static>(v: F) -> T {
        T::from(v).unwrap_or(if v > F::zero() {
            T::max_value()
        } else {
            T::min_value()
        })
    }
}

impl OverflowMode for Wrap {
    #[inline]
    fn from_float<F: Float, T: PrimInt + 'static>(v: F) -> T
    where
        i128: AsPrimitive<T>,
    {
        if v.is_nan() {
            return T::zero();
        }
        // Wrapping is defined on the integer value, so widen first. i128 covers every
        // destination type; anything that does not fit it (the infinities, and magnitudes
        // past 2^127) has no usable low bits left, so clamp instead.
        match v.to_i128() {
            Some(wide) => wide.as_(),
            None => Saturate::from_float(v),
        }
    }

    #[inline]
    fn narrow_int<F: PrimInt + AsPrimitive<T>, T: PrimInt + 'static>(v: F) -> T {
        // `AsPrimitive` is `as` semantics, which is the wrapping conversion we want here.
        // The caveat in `CastElement` about `AsPrimitive` concerns bool, which never
        // reaches this path.
        v.as_()
    }
}

impl OverflowMode for Panic {
    #[inline]
    fn from_float<F: Float, T: PrimInt + 'static>(v: F) -> T
    where
        i128: AsPrimitive<T>,
    {
        // NaN and the infinities are not representable, so they panic here too.
        T::from(v).expect("cast overflowed the destination type")
    }

    #[inline]
    fn narrow_int<F: PrimInt + AsPrimitive<T>, T: PrimInt + 'static>(v: F) -> T {
        T::from(v).expect("cast overflowed the destination type")
    }
}

/// How a float is reduced to a whole number before converting to an integer.
///
/// Only applies when casting a float to an integer; it is a no-op for every other pair.
///
/// Rounding runs before the overflow mode, so it can push a value out of the
/// destination's range: 255.6 truncates to 255 but rounds to 256, which no longer fits a
/// `u8`. `Ceiling` can do the same at the top of a range, and `Floor` at the bottom.
pub trait RoundingMode {
    fn apply<F: Float>(v: F) -> F;
}

/// Round toward zero, matching `as`.
pub struct Truncate;

/// Round to the nearest whole number, with halfway cases rounding away from zero.
pub struct Nearest;

/// Round toward negative infinity.
pub struct Floor;

/// Round toward positive infinity.
pub struct Ceiling;

impl RoundingMode for Truncate {
    #[inline]
    fn apply<F: Float>(v: F) -> F {
        v.trunc()
    }
}

impl RoundingMode for Nearest {
    #[inline]
    fn apply<F: Float>(v: F) -> F {
        v.round()
    }
}

impl RoundingMode for Floor {
    #[inline]
    fn apply<F: Float>(v: F) -> F {
        v.floor()
    }
}

impl RoundingMode for Ceiling {
    #[inline]
    fn apply<F: Float>(v: F) -> F {
        v.ceil()
    }
}

/// Converts a scalar element from one type to another, under a given overflow and
/// rounding mode.
///
/// This is deliberately not built on `num_traits::AsPrimitive` (as of 0.2.19), which has
/// incomplete bool conversion. Instead the cases are handled explicitly:
/// - converting to `bool` tests truthiness
/// - converting from `bool` maps to 0/1
/// - float to integer rounds, then applies the overflow mode
/// - integer to integer applies the overflow mode
/// - everything else is a plain `as` cast, where neither mode can apply.
///
/// The modes default to `Saturate` and `Truncate`. That reproduces `as` exactly for float
/// to integer, but deliberately differs when narrowing between integers, where `as` wraps
/// and `Saturate` clamps.
///
/// Other blocks that convert internally can still keep writing `CastElement<T>`: every one
/// of them has a float on one side of the conversion, so the defaults and `as` agree.
pub trait CastElement<O: Scalar, Ovf: OverflowMode = Saturate, Rnd: RoundingMode = Truncate>:
    Scalar
{
    fn cast_element(self) -> O;
}

/// Float -> integer. Rounds per `Rnd`, then narrows per `Ovf`.
macro_rules! impl_casts_float_to_int {
    ($from:ty => $($to:ty),* $(,)?) => {
        $(
            impl<Ovf: OverflowMode, Rnd: RoundingMode> CastElement<$to, Ovf, Rnd> for $from {
                #[inline]
                fn cast_element(self) -> $to {
                    Ovf::from_float(Rnd::apply(self))
                }
            }
        )*
    };
}

/// Integer -> integer. Narrowing is governed by `Ovf`; rounding cannot apply.
macro_rules! impl_casts_int_to_int {
    ($from:ty => $($to:ty),* $(,)?) => {
        $(
            impl<Ovf: OverflowMode, Rnd: RoundingMode> CastElement<$to, Ovf, Rnd> for $from {
                #[inline]
                fn cast_element(self) -> $to {
                    Ovf::narrow_int(self)
                }
            }
        )*
    };
}

/// Integer -> float and float -> float, done with a plain `as`. Neither mode applies:
///
/// - An integer cannot overflow a float, since every integer type's range fits inside both
///   float ranges (`u64::MAX` is about 1.8e19 against an `f32::MAX` of 3.4e38). `f64 -> f32`
///   can exceed the range, but yields an infinity rather than clamping, so there is nothing
///   for `Ovf` to act on.
/// - Precision is still lost above 2^24 for `f32` and 2^53 for `f64`, and `as` resolves that
///   by rounding to the nearest representable value, ties to even. `Rnd` does not override
///   it: `Rnd` selects how a float is reduced to a whole number, and no whole number is
///   involved here. Rust offers no directed-rounding conversion to implement it with anyway.
macro_rules! impl_casts_as {
    ($from:ty => $($to:ty),* $(,)?) => {
        $(
            impl<Ovf: OverflowMode, Rnd: RoundingMode> CastElement<$to, Ovf, Rnd> for $from {
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
            impl<Ovf: OverflowMode, Rnd: RoundingMode> CastElement<bool, Ovf, Rnd> for $from {
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
            impl<Ovf: OverflowMode, Rnd: RoundingMode> CastElement<$to, Ovf, Rnd> for bool {
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
impl_casts_int_to_int!(u8 => u8, i8, u16, i16, u32, i32, u64, i64);
impl_casts_int_to_int!(i8 => u8, i8, u16, i16, u32, i32, u64, i64);
impl_casts_int_to_int!(u16 => u8, i8, u16, i16, u32, i32, u64, i64);
impl_casts_int_to_int!(i16 => u8, i8, u16, i16, u32, i32, u64, i64);
impl_casts_int_to_int!(u32 => u8, i8, u16, i16, u32, i32, u64, i64);
impl_casts_int_to_int!(i32 => u8, i8, u16, i16, u32, i32, u64, i64);
impl_casts_int_to_int!(u64 => u8, i8, u16, i16, u32, i32, u64, i64);
impl_casts_int_to_int!(i64 => u8, i8, u16, i16, u32, i32, u64, i64);

impl_casts_float_to_int!(f32 => u8, i8, u16, i16, u32, i32, u64, i64);
impl_casts_float_to_int!(f64 => u8, i8, u16, i16, u32, i32, u64, i64);

impl_casts_as!(u8 => f32, f64);
impl_casts_as!(i8 => f32, f64);
impl_casts_as!(u16 => f32, f64);
impl_casts_as!(i16 => f32, f64);
impl_casts_as!(u32 => f32, f64);
impl_casts_as!(i32 => f32, f64);
impl_casts_as!(u64 => f32, f64);
impl_casts_as!(i64 => f32, f64);
impl_casts_as!(f32 => f32, f64);
impl_casts_as!(f64 => f32, f64);

impl_casts_to_bool!(u8, i8, u16, i16, u32, i32, u64, i64, f32, f64);
impl_casts_from_bool!(u8, i8, u16, i16, u32, i32, u64, i64, f32, f64);

impl<Ovf: OverflowMode, Rnd: RoundingMode> CastElement<bool, Ovf, Rnd> for bool {
    #[inline]
    fn cast_element(self) -> bool {
        self
    }
}

/// A signal that can be converted to signal type `O`, preserving shape.
///
/// Implemented for scalars and matrices of the same dimensions.
pub trait CastTo<O: Pass + Default, Ovf: OverflowMode = Saturate, Rnd: RoundingMode = Truncate>:
    Pass + Default
{
    fn cast<'a>(input: PassBy<'_, Self>, dest: &'a mut O) -> PassBy<'a, O>;
}

impl<I, O, Ovf, Rnd> CastTo<O, Ovf, Rnd> for I
where
    I: Scalar + CastElement<O, Ovf, Rnd>,
    O: Scalar,
    Ovf: OverflowMode,
    Rnd: RoundingMode,
{
    fn cast<'a>(input: PassBy<'_, Self>, dest: &'a mut O) -> PassBy<'a, O> {
        *dest = input.cast_element();
        *dest
    }
}

impl<const ROWS: usize, const COLS: usize, I, O, Ovf, Rnd> CastTo<Matrix<ROWS, COLS, O>, Ovf, Rnd>
    for Matrix<ROWS, COLS, I>
where
    I: Scalar + CastElement<O, Ovf, Rnd>,
    O: Scalar,
    Ovf: OverflowMode,
    Rnd: RoundingMode,
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
///
/// `Ovf` and `Rnd` select the overflow and rounding behavior, defaulting to `Saturate`
/// and `Truncate`. That matches `as` for float to integer, but clamps rather than wrapping
/// when narrowing between integers.
pub struct CastBlock<I, O, Ovf = Saturate, Rnd = Truncate>
where
    I: CastTo<O, Ovf, Rnd>,
    O: Pass + Default,
    Ovf: OverflowMode,
    Rnd: RoundingMode,
{
    buffer: O,
    phantom: core::marker::PhantomData<(I, Ovf, Rnd)>,
}

// Derived Default would demand Default on the mode markers, which are only ever phantom.
impl<I, O, Ovf, Rnd> Default for CastBlock<I, O, Ovf, Rnd>
where
    I: CastTo<O, Ovf, Rnd>,
    O: Pass + Default,
    Ovf: OverflowMode,
    Rnd: RoundingMode,
{
    fn default() -> Self {
        Self {
            buffer: O::default(),
            phantom: core::marker::PhantomData,
        }
    }
}

impl<I, O, Ovf, Rnd> ProcessBlock for CastBlock<I, O, Ovf, Rnd>
where
    I: CastTo<O, Ovf, Rnd>,
    O: Pass + Default,
    Ovf: OverflowMode,
    Rnd: RoundingMode,
{
    type Parameters = Parameters;
    type Inputs = I;
    type Output = O;

    fn process<'b>(
        &'b mut self,
        _parameters: &Self::Parameters,
        _model_clock: &dyn ModelClock,
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
    use crate::testing::StubModelClock;

    fn cast_scalar<I, O>(input: I) -> O
    where
        I: CastTo<O> + Scalar,
        O: Scalar,
    {
        let mut block = CastBlock::<I, O>::default();
        let model_clock = StubModelClock::default();
        block.process(&Parameters::new(), &model_clock, input)
    }

    /// `cast_scalar` under an explicit overflow and rounding mode.
    fn cast_scalar_as<I, O, Ovf, Rnd>(input: I) -> O
    where
        I: CastTo<O, Ovf, Rnd> + Scalar,
        O: Scalar,
        Ovf: OverflowMode,
        Rnd: RoundingMode,
    {
        let mut block = CastBlock::<I, O, Ovf, Rnd>::default();
        let model_clock = StubModelClock::default();
        block.process(&Parameters::new(), &model_clock, input)
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
    fn test_int_to_int_saturates_by_default() {
        // The default mode clamps rather than wrapping, which is the one way the default
        // differs from a plain `as`. Wrap is still available explicitly.
        assert_eq!(cast_scalar::<u16, u8>(300), 255u8);
        assert_eq!(cast_scalar::<i32, u8>(-1), 0u8);
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
        let model_clock = StubModelClock::default();

        let input = Matrix::<2, 2, f64> {
            data: [[1.9, 2.1], [300.0, -1.0]],
        };
        let expected = Matrix::<2, 2, u8> {
            data: [[1, 2], [255, 0]],
        };

        assert_eq!(
            block.process(&Parameters::new(), &model_clock, &input),
            &expected
        );
        assert_eq!(block.buffer(), &expected);
    }

    #[test]
    fn test_matrix_cast_to_bool() {
        let mut block = CastBlock::<Matrix<1, 3, f64>, Matrix<1, 3, bool>>::default();
        let model_clock = StubModelClock::default();

        let input = Matrix::<1, 3, f64> {
            data: [[0.0], [1.0], [-2.5]],
        };
        let output = block.process(&Parameters::new(), &model_clock, &input);

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
        let model_clock = StubModelClock::default();

        let input = Matrix::<3, 1, i32> { data: [[-1, 0, 5]] };
        let output = block.process(&Parameters::new(), &model_clock, &input);

        assert_eq!(
            output,
            &Matrix::<3, 1, f32> {
                data: [[-1.0, 0.0, 5.0]],
            }
        );
    }

    #[test]
    fn test_int_to_int_overflow_modes() {
        assert_eq!(cast_scalar_as::<u16, u8, Saturate, Truncate>(300), 255u8);
        assert_eq!(cast_scalar_as::<u16, u8, Wrap, Truncate>(300), 44u8);

        assert_eq!(cast_scalar_as::<i32, u8, Saturate, Truncate>(-1), 0u8);
        assert_eq!(cast_scalar_as::<i32, u8, Wrap, Truncate>(-1), 255u8);

        // In range under every mode.
        assert_eq!(cast_scalar_as::<u16, u8, Saturate, Truncate>(42), 42u8);
        assert_eq!(cast_scalar_as::<u16, u8, Wrap, Truncate>(42), 42u8);
        assert_eq!(cast_scalar_as::<u16, u8, Panic, Truncate>(42), 42u8);
    }

    #[test]
    #[should_panic(expected = "cast overflowed the destination type")]
    fn test_int_to_int_panic_mode() {
        cast_scalar_as::<u16, u8, Panic, Truncate>(300);
    }

    #[test]
    fn test_signed_to_unsigned_only_overflows_when_negative() {
        // Every non-negative i8 fits a u8, so the modes agree and Panic has nothing to
        // fire on. Only a negative value overflows.
        for mode_result in [
            cast_scalar_as::<i8, u8, Panic, Truncate>(100),
            cast_scalar_as::<i8, u8, Wrap, Truncate>(100),
            cast_scalar_as::<i8, u8, Saturate, Truncate>(100),
        ] {
            assert_eq!(mode_result, 100u8);
        }

        assert_eq!(cast_scalar_as::<i8, u8, Wrap, Truncate>(-1), 255u8);
        assert_eq!(cast_scalar_as::<i8, u8, Saturate, Truncate>(-1), 0u8);
    }

    #[test]
    #[should_panic(expected = "cast overflowed the destination type")]
    fn test_signed_to_unsigned_panics_on_negative() {
        cast_scalar_as::<i8, u8, Panic, Truncate>(-1);
    }

    #[test]
    fn test_float_to_int_overflow_modes() {
        // Saturate clamps to the nearest limit, for signed and unsigned destinations.
        assert_eq!(cast_scalar_as::<f64, u8, Saturate, Truncate>(300.7), 255u8);
        assert_eq!(cast_scalar_as::<f64, u8, Saturate, Truncate>(-1.2), 0u8);
        assert_eq!(
            cast_scalar_as::<f64, i8, Saturate, Truncate>(-500.7),
            -128i8
        );
        assert_eq!(cast_scalar_as::<f64, i8, Saturate, Truncate>(500.7), 127i8);

        // Wrap keeps the low bits of the rounded value, so the rounding mode changes the
        // result: 300.7 truncates to 300 (-> 44) but rounds to 301 (-> 45).
        assert_eq!(cast_scalar_as::<f64, u8, Wrap, Truncate>(300.7), 44u8);
        assert_eq!(cast_scalar_as::<f64, u8, Wrap, Nearest>(300.7), 45u8);
        assert_eq!(cast_scalar_as::<f64, u8, Wrap, Truncate>(-1.2), 255u8);

        // Wrapping into a signed type crosses the sign bit rather than just exceeding a
        // magnitude: 200 has bit 7 set, so it reads as -56 in an i8.
        assert_eq!(cast_scalar_as::<f64, i8, Wrap, Truncate>(200.7), -56i8);
        assert_eq!(cast_scalar_as::<f64, i8, Wrap, Truncate>(-200.7), 56i8);
    }

    #[test]
    #[should_panic(expected = "cast overflowed the destination type")]
    fn test_float_to_int_panic_mode() {
        cast_scalar_as::<f64, u8, Panic, Truncate>(300.0);
    }

    #[test]
    fn test_rounding_modes() {
        // Truncate rounds toward zero, Floor toward negative infinity; they differ only
        // for negatives. Nearest breaks halfway cases away from zero.
        assert_eq!(cast_scalar_as::<f64, i32, Saturate, Truncate>(2.7), 2);
        assert_eq!(cast_scalar_as::<f64, i32, Saturate, Nearest>(2.7), 3);
        assert_eq!(cast_scalar_as::<f64, i32, Saturate, Floor>(2.7), 2);

        assert_eq!(cast_scalar_as::<f64, i32, Saturate, Ceiling>(2.7), 3);

        assert_eq!(cast_scalar_as::<f64, i32, Saturate, Truncate>(-2.7), -2);
        assert_eq!(cast_scalar_as::<f64, i32, Saturate, Nearest>(-2.7), -3);
        assert_eq!(cast_scalar_as::<f64, i32, Saturate, Floor>(-2.7), -3);
        assert_eq!(cast_scalar_as::<f64, i32, Saturate, Ceiling>(-2.7), -2);

        assert_eq!(cast_scalar_as::<f64, i32, Saturate, Nearest>(2.5), 3);
        assert_eq!(cast_scalar_as::<f64, i32, Saturate, Nearest>(-2.5), -3);

        // Truncate matches Ceiling for negatives and Floor for positives, since all three
        // agree on which way "toward zero" points for a given sign.
        assert_eq!(cast_scalar_as::<f64, i32, Saturate, Floor>(2.7), 2);
        assert_eq!(cast_scalar_as::<f64, i32, Saturate, Ceiling>(-2.7), -2);
    }

    #[test]
    fn test_fractional_out_of_range_floats() {
        // Rounding runs first, then the overflow mode acts on the rounded value.
        assert_eq!(
            cast_scalar_as::<f64, u8, Saturate, Truncate>(257.234),
            255u8
        );
        assert_eq!(cast_scalar_as::<f64, u8, Saturate, Nearest>(257.234), 255u8);
        // 257.234 truncates to 257, which wraps to 1.
        assert_eq!(cast_scalar_as::<f64, u8, Wrap, Truncate>(257.234), 1u8);
        assert_eq!(cast_scalar_as::<f64, u8, Saturate, Truncate>(-0.4), 0u8);
    }

    #[test]
    fn test_rounding_can_push_a_value_out_of_range() {
        // 255.6 truncates into range but rounds out of it, so the rounding mode decides
        // whether the overflow mode sees an out-of-range value at all.
        assert_eq!(cast_scalar_as::<f64, u8, Panic, Truncate>(255.6), 255u8);
        assert_eq!(cast_scalar_as::<f64, u8, Saturate, Nearest>(255.6), 255u8);

        // Under Wrap the same input differs by rounding mode: 255 stays, 256 wraps to 0.
        assert_eq!(cast_scalar_as::<f64, u8, Wrap, Truncate>(255.6), 255u8);
        assert_eq!(cast_scalar_as::<f64, u8, Wrap, Nearest>(255.6), 0u8);
    }

    #[test]
    #[should_panic(expected = "cast overflowed the destination type")]
    fn test_rounding_out_of_range_panics_under_panic_mode() {
        cast_scalar_as::<f64, u8, Panic, Nearest>(255.6);
    }

    #[test]
    fn test_rounding_does_not_apply_to_integer_sources() {
        assert_eq!(cast_scalar_as::<i32, i64, Saturate, Nearest>(-7), -7i64);
        assert_eq!(cast_scalar_as::<i32, f64, Saturate, Floor>(-7), -7.0);
        assert_eq!(cast_scalar_as::<i32, i64, Saturate, Ceiling>(-7), -7i64);
    }

    #[test]
    fn test_ceiling_can_push_a_value_out_of_range() {
        // Ceiling rounds up, so it overflows the top of a range that Truncate stays inside:
        // 255.2 truncates to 255 and fits, but ceils to 256 and does not.
        assert_eq!(cast_scalar_as::<f64, u8, Saturate, Truncate>(255.2), 255u8);
        assert_eq!(cast_scalar_as::<f64, u8, Saturate, Ceiling>(255.2), 255u8);
        assert_eq!(cast_scalar_as::<f64, u8, Wrap, Ceiling>(255.2), 0u8);

        // Floor is the mirror at the bottom of a signed range: -128.2 truncates to -128
        // and fits, but floors to -129 and does not.
        assert_eq!(
            cast_scalar_as::<f64, i8, Saturate, Truncate>(-128.2),
            -128i8
        );
        assert_eq!(cast_scalar_as::<f64, i8, Saturate, Floor>(-128.2), -128i8);
        assert_eq!(cast_scalar_as::<f64, i8, Wrap, Floor>(-128.2), 127i8);
    }

    #[test]
    #[should_panic(expected = "cast overflowed the destination type")]
    fn test_ceiling_out_of_range_panics_under_panic_mode() {
        cast_scalar_as::<f64, u8, Panic, Ceiling>(255.2);
    }

    #[test]
    fn test_non_finite_floats() {
        // Saturate and Wrap follow `as`: NaN is zero, the infinities clamp.
        assert_eq!(cast_scalar_as::<f64, i32, Saturate, Truncate>(f64::NAN), 0);
        assert_eq!(cast_scalar_as::<f64, i32, Wrap, Truncate>(f64::NAN), 0);
        assert_eq!(
            cast_scalar_as::<f64, i32, Saturate, Truncate>(f64::INFINITY),
            i32::MAX
        );
        assert_eq!(
            cast_scalar_as::<f64, i32, Saturate, Truncate>(f64::NEG_INFINITY),
            i32::MIN
        );

        // The infinities have no low bits to wrap, so Wrap falls back to clamping.
        assert_eq!(
            cast_scalar_as::<f64, i32, Wrap, Truncate>(f64::INFINITY),
            i32::MAX
        );
        assert_eq!(
            cast_scalar_as::<f64, i32, Wrap, Truncate>(f64::NEG_INFINITY),
            i32::MIN
        );
    }

    #[test]
    #[should_panic(expected = "cast overflowed the destination type")]
    fn test_nan_panics_under_panic_mode() {
        cast_scalar_as::<f64, i32, Panic, Truncate>(f64::NAN);
    }

    #[test]
    fn test_modes_apply_elementwise_to_matrices() {
        let mut block =
            CastBlock::<Matrix<3, 1, f64>, Matrix<3, 1, u8>, Saturate, Nearest>::default();
        let model_clock = StubModelClock::default();

        let input = Matrix::<3, 1, f64> {
            data: [[2.5, -1.0, 300.0]],
        };
        let output = block.process(&Parameters::new(), &model_clock, &input);

        assert_eq!(
            output,
            &Matrix::<3, 1, u8> {
                data: [[3, 0, 255]]
            }
        );
    }
}
