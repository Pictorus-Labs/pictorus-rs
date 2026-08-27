use super::cast_block::CastElement;
use crate::traits::{Float, Scalar};
use pictorus_traits::{GeneratorBlock, PassBy};

#[derive(Debug, Clone)]
/// Outputs a sinewave signal with specified amplitude, frequency, phase, and bias.
///
/// The wave is computed in the float type `F` and cast to the output type `T` with
/// `as` semantics. Value-domain parameters (amplitude, bias) are specified in `T`;
/// time-domain parameters (frequency, phase) in `F`.
pub struct SinewaveBlock<T, F = T>
where
    T: Scalar + CastElement<F>,
    F: Float + CastElement<T>,
    f64: From<F>,
{
    phantom: core::marker::PhantomData<F>,
    buffer: T,
}

impl<T, F> Default for SinewaveBlock<T, F>
where
    T: Scalar + CastElement<F>,
    F: Float + CastElement<T>,
    f64: From<F>,
{
    fn default() -> Self {
        Self {
            phantom: core::marker::PhantomData,
            buffer: T::default(),
        }
    }
}

impl<T, F> GeneratorBlock for SinewaveBlock<T, F>
where
    T: Scalar + CastElement<F>,
    F: Float + CastElement<T>,
    f64: From<F>,
{
    type Parameters = Parameters<T, F>;
    type Output = T;

    fn generate(
        &mut self,
        parameters: &Self::Parameters,
        context: &dyn pictorus_traits::Context,
    ) -> pictorus_traits::PassBy<'_, Self::Output> {
        let amplitude: F = parameters.amplitude.cast_element();
        let bias: F = parameters.bias.cast_element();
        let time = F::from_duration(context.time());
        let sin_val = amplitude
            * num_traits::Float::sin(parameters.frequency * time + parameters.phase)
            + bias;
        self.buffer = sin_val.cast_element();
        self.buffer
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        self.buffer
    }
}

#[derive(Debug, Clone)]
pub struct Parameters<T: Scalar, F: Float = T> {
    pub amplitude: T,
    pub frequency: F,
    pub phase: F,
    pub bias: T,
}

impl<T: Scalar, F: Float> Parameters<T, F> {
    pub fn new(amplitude: T, frequency: F, phase: F, bias: T) -> Self {
        Self {
            amplitude,
            frequency,
            phase,
            bias,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::testing::StubContext;
    use core::time::Duration;
    use num_traits::Float;

    #[test]
    fn test_sinewave_default_buffer_no_panic() {
        let block = SinewaveBlock::<f64>::default();
        assert_eq!(block.buffer(), 0.0);
    }

    #[test]
    fn test_sine_wave() {
        let mut block = SinewaveBlock::<f64>::default();
        let parameters = Parameters {
            amplitude: 1.0,
            frequency: 1.0,
            phase: 0.5,
            bias: 0.0,
        };

        let mut context = StubContext::default();

        assert_eq!(block.generate(&parameters, &context), Float::sin(0.5));
        assert_eq!(block.buffer(), Float::sin(0.5));
        context.time = Duration::from_secs(1);

        assert_eq!(block.generate(&parameters, &context), Float::sin(1.5));
        assert_eq!(block.buffer(), Float::sin(1.5));
    }

    #[test]
    fn test_sine_wave_integer_output() {
        // u8 output: computed in f64, cast with `as` semantics at the output.
        let mut block = SinewaveBlock::<u8, f64>::default();
        let parameters = Parameters::new(100u8, 1.0, 0.0, 100u8);

        let mut context = StubContext::default();

        // t = 0: 100 * sin(0) + 100 = 100
        assert_eq!(block.generate(&parameters, &context), 100);

        // sin peaks near t = pi/2: 100 * ~1.0 + 100 ~= 200 truncated
        context.time = Duration::from_secs_f64(core::f64::consts::FRAC_PI_2);
        assert_eq!(block.generate(&parameters, &context), 200);

        // trough near t = 3pi/2: 100 * ~-1.0 + 100 ~= 0 (not wrapped)
        context.time = Duration::from_secs_f64(3.0 * core::f64::consts::FRAC_PI_2);
        assert_eq!(block.generate(&parameters, &context), 0);
    }

    #[test]
    fn test_sine_wave_integer_output_saturates() {
        // Negative trough saturates at 0 for an unsigned output rather than wrapping.
        let mut block = SinewaveBlock::<u8, f64>::default();
        let parameters = Parameters::new(100u8, 1.0, 0.0, 0u8);

        let context = StubContext {
            time: Duration::from_secs_f64(3.0 * core::f64::consts::FRAC_PI_2),
            ..Default::default()
        };
        assert_eq!(block.generate(&parameters, &context), 0);
    }

    // Limits of the compute-in-float strategy. None of these panic: float arithmetic
    // never panics and the output cast saturates, so this block has no arithmetic
    // panic sites (relevant when auditing overflow behavior for the app-level
    // settings work).

    #[test]
    fn test_sine_wave_amplitude_beyond_float_precision_rounds() {
        // A u64 amplitude above 2^53 is not exactly representable in the f64 the wave
        // is computed in: 2^53 + 1 silently rounds to 2^53 on the way through.
        // frequency = 0 with phase = pi/2 pins the wave at its peak (sin = exactly 1.0
        // in f64), isolating the round trip.
        let mut block = SinewaveBlock::<u64, f64>::default();
        let parameters = Parameters::new((1u64 << 53) + 1, 0.0, core::f64::consts::FRAC_PI_2, 0u64);

        let context = StubContext::default();
        assert_eq!(block.generate(&parameters, &context), 1u64 << 53);
    }

    #[test]
    fn test_sine_wave_nan_becomes_zero_for_integer_output() {
        // A NaN parameter poisons the float math; the output cast turns NaN into 0
        // rather than panicking (`as` semantics).
        let mut block = SinewaveBlock::<u8, f64>::default();
        let parameters = Parameters::new(100u8, 0.0, f64::NAN, 100u8);

        let context = StubContext::default();
        assert_eq!(block.generate(&parameters, &context), 0);
    }
}
