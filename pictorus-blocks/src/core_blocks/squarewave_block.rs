use crate::traits::{Float, Scalar};
use pictorus_traits::{GeneratorBlock, PassBy};

/// Combines a wave's amplitude with its bias in the output's native type.
///
/// The square wave's output values never pass through a float, so the "high" level is
/// formed directly in the output type: plain addition for numeric types,
//  and logical OR for bools.
///
/// Integer numeric types have the possibility of panic-ing if the bias + amplitude
// overflow the type's bounds
pub trait ApplyBias: Scalar {
    fn apply_bias(self, bias: Self) -> Self;
}

macro_rules! impl_apply_bias_numeric {
    ($($t:ty),* $(,)?) => {
        $(
            impl ApplyBias for $t {
                #[inline]
                fn apply_bias(self, bias: Self) -> Self {
                    self + bias
                }
            }
        )*
    };
}

impl_apply_bias_numeric!(u8, i8, u16, i16, u32, i32, u64, i64, f32, f64);

impl ApplyBias for bool {
    #[inline]
    fn apply_bias(self, bias: Self) -> Self {
        self | bias
    }
}

pub struct Parameters<T: Scalar, F: Float = T> {
    pub amplitude: T,
    pub on_duration: F,
    pub off_duration: F,
    pub phase: F,
    pub bias: T,
}

impl<T: Scalar, F: Float> Parameters<T, F> {
    pub fn new(amplitude: T, on_duration: F, off_duration: F, phase: F, bias: T) -> Self {
        Self {
            amplitude,
            on_duration,
            off_duration,
            phase,
            bias,
        }
    }
}

/// Outputs a square wave signal with specified amplitude, on duration, off duration, phase, and bias.
///
/// The on/off timing is computed in the float type `F`, but the output levels are formed
/// natively in the output type `T` (`bias` when off, `bias ⊕ amplitude` when on) — no
/// cast is involved. Value-domain parameters (amplitude, bias) are specified in `T`;
/// time-domain parameters (on_duration, off_duration, phase) in `F`.
pub struct SquarewaveBlock<T: Scalar + ApplyBias, F: Float = T> {
    phantom: core::marker::PhantomData<F>,
    buffer: T,
}

impl<T, F> Default for SquarewaveBlock<T, F>
where
    T: Scalar + ApplyBias,
    F: Float,
    f64: From<F>,
{
    fn default() -> Self {
        Self {
            phantom: core::marker::PhantomData,
            buffer: T::default(),
        }
    }
}

impl<T, F> GeneratorBlock for SquarewaveBlock<T, F>
where
    T: Scalar + ApplyBias,
    F: Float,
    f64: From<F>,
{
    type Output = T;
    type Parameters = Parameters<T, F>;

    fn generate(
        &mut self,
        parameters: &Self::Parameters,
        model_clock: &dyn pictorus_traits::ModelClock,
    ) -> pictorus_traits::PassBy<'_, Self::Output> {
        let adjusted_time = F::from_duration(model_clock.time()) - parameters.phase;
        let pulse_time = parameters.on_duration + parameters.off_duration;
        let mut time_since_last_pulse_start: F = adjusted_time % pulse_time;

        if time_since_last_pulse_start < F::zero() {
            // Adjust for negative phase
            time_since_last_pulse_start += pulse_time
        };

        let output = if time_since_last_pulse_start > parameters.on_duration {
            parameters.bias
        } else {
            parameters.amplitude.apply_bias(parameters.bias)
        };
        self.buffer = output;
        output
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        self.buffer
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::StubRuntime;

    use super::*;
    use core::time::Duration;

    #[test]
    fn test_squarewave_default_buffer_no_panic() {
        let block = SquarewaveBlock::<f64>::default();
        assert_eq!(block.buffer(), 0.0);
    }

    #[test]
    fn test_squarewave_block_f64() {
        let amplitude = 2.0;
        let on_duration = 1.0;
        let off_duration = 2.0;
        let bias = 0.25;
        let phase = 0.5;

        let p = Parameters::new(amplitude, on_duration, off_duration, phase, bias);

        let mut block = SquarewaveBlock::<f64>::default();

        let mut runtime = StubRuntime::default();

        block.generate(&p, &runtime.model_clock());
        assert_eq!(block.generate(&p, &runtime.model_clock()), bias);
        assert_eq!(block.buffer(), bias);

        runtime.set_time(Duration::from_millis(500));
        assert_eq!(block.generate(&p, &runtime.model_clock()), bias + amplitude);
        assert_eq!(block.buffer(), bias + amplitude);

        runtime.set_time(Duration::from_secs_f64(1.0));
        assert_eq!(block.generate(&p, &runtime.model_clock()), bias + amplitude);
        assert_eq!(block.buffer(), bias + amplitude);

        runtime.set_time(Duration::from_secs_f64(1.499));
        assert_eq!(block.generate(&p, &runtime.model_clock()), bias + amplitude);
        assert_eq!(block.buffer(), bias + amplitude);

        runtime.set_time(Duration::from_secs_f64(1.5));
        assert_eq!(block.generate(&p, &runtime.model_clock()), bias + amplitude);
        assert_eq!(block.buffer(), bias + amplitude);

        // Off duration
        runtime.set_time(Duration::from_secs_f64(2.5));
        assert_eq!(block.generate(&p, &runtime.model_clock()), bias);
        assert_eq!(block.buffer(), bias);

        runtime.set_time(Duration::from_secs_f64(3.499));
        assert_eq!(block.generate(&p, &runtime.model_clock()), bias);
        assert_eq!(block.buffer(), bias);

        // Back on
        runtime.set_time(Duration::from_secs_f64(3.5));
        assert_eq!(block.generate(&p, &runtime.model_clock()), bias + amplitude);
        assert_eq!(block.buffer(), bias + amplitude);
    }

    #[test]
    fn test_squarewave_block_f32() {
        let amplitude = 2.0;
        let on_duration = 1.0;
        let off_duration = 2.0;
        let bias = 0.5;
        let phase = 0.5;

        let p = Parameters::new(amplitude, on_duration, off_duration, phase, bias);

        let mut block = SquarewaveBlock::<f32>::default();

        let mut runtime = StubRuntime::default();

        block.generate(&p, &runtime.model_clock());
        assert_eq!(block.generate(&p, &runtime.model_clock()), bias);
        assert_eq!(block.buffer(), bias);

        runtime.set_time(Duration::from_millis(500));
        assert_eq!(block.generate(&p, &runtime.model_clock()), bias + amplitude);
        assert_eq!(block.buffer(), bias + amplitude);

        runtime.set_time(Duration::from_secs_f32(1.0));
        assert_eq!(block.generate(&p, &runtime.model_clock()), bias + amplitude);
        assert_eq!(block.buffer(), bias + amplitude);

        runtime.set_time(Duration::from_secs_f32(1.499));
        assert_eq!(block.generate(&p, &runtime.model_clock()), bias + amplitude);
        assert_eq!(block.buffer(), bias + amplitude);

        runtime.set_time(Duration::from_secs_f32(1.5));
        assert_eq!(block.generate(&p, &runtime.model_clock()), bias + amplitude);
        assert_eq!(block.buffer(), bias + amplitude);

        // Off duration
        runtime.set_time(Duration::from_secs_f32(2.5));
        assert_eq!(block.generate(&p, &runtime.model_clock()), bias);
        assert_eq!(block.buffer(), bias);

        runtime.set_time(Duration::from_secs_f32(3.499));
        assert_eq!(block.generate(&p, &runtime.model_clock()), bias);
        assert_eq!(block.buffer(), bias);

        // Back on
        runtime.set_time(Duration::from_secs_f32(3.5));
        assert_eq!(block.generate(&p, &runtime.model_clock()), bias + amplitude);
        assert_eq!(block.buffer(), bias + amplitude);
    }

    #[test]
    fn test_squarewave_phase() {
        // Shout out to Jason for spotting this gap in testing
        let amplitude = 1.0;
        let on_duration = 1.0;
        let off_duration = 2.0;
        let bias = 0.0;
        let phase = 1.5;

        let mut p = Parameters::new(amplitude, on_duration, off_duration, phase, bias);

        let mut block = SquarewaveBlock::<f32>::default();

        let runtime = StubRuntime::default();

        p.phase = 1.5;
        block.generate(&p, &runtime.model_clock());
        assert_eq!(block.generate(&p, &runtime.model_clock()), bias);

        p.phase = 0.0;
        assert_eq!(block.generate(&p, &runtime.model_clock()), bias + amplitude);

        p.phase = 0.5;
        assert_eq!(block.generate(&p, &runtime.model_clock()), bias);

        p.phase = 2.5;
        assert_eq!(block.generate(&p, &runtime.model_clock()), bias + amplitude);

        // No Phase Shift:
        //
        //-----               |--------               ----------
        //                    |
        //                    |
        //     ---------------|        ----------------
        //-----------------------------------------------------
        //   -2      -1       0       1       2       3       4

        // 2.5 Phase Shift:
        //
        //-----           ----|----                ----------
        //                    |
        //                    |
        //     -----------    |    ----------------
        //-----------------------------------------------------
        //   -2      -1       0       1       2       3       4
    }

    #[test]
    fn test_squarewave_bias() {
        let amplitude = 1.0;
        let on_duration = 1.0;
        let off_duration = 2.0;
        let bias = 1.0;
        let phase = 0.0;

        let p = Parameters::new(amplitude, on_duration, off_duration, phase, bias);
        let mut block = SquarewaveBlock::<f32>::default();

        let mut runtime = StubRuntime::default();

        let output = block.generate(&p, &runtime.model_clock());
        assert_eq!(output, bias + amplitude);

        runtime.set_time(Duration::from_secs_f32(1.5));
        let output = block.generate(&p, &runtime.model_clock());
        assert_eq!(output, bias);
    }

    #[test]
    fn test_squarewave_block_u8() {
        // Native integer output and computation
        let p = Parameters::<u8, f64>::new(200, 1.0, 1.0, 0.0, 20);
        let mut block = SquarewaveBlock::<u8, f64>::default();

        let mut runtime = StubRuntime::default();

        // On: amplitude 200 + bias 20
        assert_eq!(block.generate(&p, &runtime.model_clock()), 220);

        // Off: bias alone.
        runtime.set_time(Duration::from_secs_f64(1.5));
        assert_eq!(block.generate(&p, &runtime.model_clock()), 20);
    }

    #[test]
    #[should_panic]
    fn test_squarewave_block_signed_overflow() {
        // ApplyBias can panic in debug builds with native addition
        let runtime = StubRuntime::default();
        let p = Parameters::<i8, f64>::new(100, 1.0, 1.0, 0.0, 100);
        let mut block = SquarewaveBlock::<i8, f64>::default();
        let _ = block.generate(&p, &runtime.model_clock());
    }

    #[test]
    #[should_panic]
    fn test_squarewave_block_signed_underflow() {
        // ApplyBias can panic in debug builds with native addition
        let mut runtime = StubRuntime::default();

        let p = Parameters::<i8, f64>::new(-100, 1.0, 1.0, 0.0, -100);
        let mut block = SquarewaveBlock::<i8, f64>::default();
        runtime.set_time(Duration::from_secs_f64(0.5));
        let _ = block.generate(&p, &runtime.model_clock());
    }

    #[test]
    fn test_squarewave_block_float_overflow_is_infinity() {
        // For float outputs ApplyBias is plain addition, which overflows to infinity
        // rather than panicking.
        let p = Parameters::<f32, f32>::new(f32::MAX, 1.0, 1.0, 0.0, f32::MAX);
        let mut block = SquarewaveBlock::<f32>::default();

        let runtime = StubRuntime::default();
        assert_eq!(block.generate(&p, &runtime.model_clock()), f32::INFINITY);
    }

    #[test]
    fn test_squarewave_block_bool() {
        // Bool output: true while on, false while off.
        let p = Parameters::<bool, f64>::new(true, 1.0, 1.0, 0.0, false);
        let mut block = SquarewaveBlock::<bool, f64>::default();

        let mut runtime = StubRuntime::default();

        assert!(block.generate(&p, &runtime.model_clock()));

        runtime.set_time(Duration::from_secs_f64(1.5));
        assert!(!block.generate(&p, &runtime.model_clock()));

        runtime.set_time(Duration::from_secs_f64(2.5));
        assert!(block.generate(&p, &runtime.model_clock()));
    }
}
