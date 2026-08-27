use super::cast_block::CastElement;
use crate::traits::{Float, Scalar};
use pictorus_traits::{GeneratorBlock, PassBy};

#[derive(Debug, Clone)]
/// Outputs a triangle wave signal with specified amplitude, frequency, phase, and bias.
///
/// The wave is computed in the float type `F` and cast to the output type `T` with
/// `as` semantics. Value-domain parameters (amplitude, bias) are specified in `T`;
/// time-domain parameters (frequency, phase) in `F`.
pub struct TrianglewaveBlock<T, F = T>
where
    T: Scalar + CastElement<F>,
    F: Float + CastElement<T>,
    f64: From<F>,
{
    phantom: core::marker::PhantomData<F>,
    buffer: T,
}

impl<T, F> Default for TrianglewaveBlock<T, F>
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

impl<T, F> GeneratorBlock for TrianglewaveBlock<T, F>
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
        // These two variables are used to construct constants used in the math below in a way that is infallible and generic
        let two: F = F::one() + F::one();
        let four: F = two + two;
        let amplitude: F = parameters.amplitude.cast_element();
        let bias: F = parameters.bias.cast_element();
        let t =
            (parameters.frequency * F::from_duration(context.time()) + parameters.phase) / (F::TAU);
        let t = num_traits::Float::fract(t);
        let y = if t < F::one() / two { t } else { F::one() - t };
        // y is in the range [0, 0.5] over a t value from 0 to 1. Scale it by 4 ( to a range of [0, 2] )
        // then shift it down by 1 to get it in the range [-1, 1], then scale it by the amplitude and add the bias.
        let val = (four * y - F::one()) * amplitude + bias;
        self.buffer = val.cast_element();
        self.buffer
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        self.buffer
    }
}

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
    use crate::testing::{StubContext, StubRuntime};
    use approx::assert_relative_eq;
    use core::time::Duration;

    const PI: f64 = core::f64::consts::PI;

    #[test]
    fn test_trianglewave_default_buffer_no_panic() {
        let block = TrianglewaveBlock::<f64>::default();
        assert_eq!(block.buffer(), 0.0);
    }

    #[test]
    fn test_trianglewave_block_simple() {
        let context = StubContext::new(
            Duration::from_secs(0),
            None,
            Duration::from_secs_f64(PI / 2.0),
        );
        let mut runtime = StubRuntime::new(context);

        let amplitude = 1.0;
        let frequency = 1.0;
        let phase = 0.0;
        let bias = 0.0;
        let params = Parameters::new(amplitude, frequency, phase, bias);

        let mut block = TrianglewaveBlock::default();

        let out = block.generate(&params, &runtime.context()); // T = 0
        assert_relative_eq!(block.buffer(), -1.0, epsilon = 1e-6);
        assert_eq!(block.buffer(), out);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI / 2
        assert_relative_eq!(block.buffer(), 0.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI
        assert_relative_eq!(block.buffer(), 1.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 3PI / 2
        assert_relative_eq!(block.buffer(), 0.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 2PI
        assert_relative_eq!(block.buffer(), -1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_trianglewave_block_phase() {
        let context = StubContext::new(
            Duration::from_secs(0),
            None,
            Duration::from_secs_f64(PI / 2.0),
        );
        let mut runtime = StubRuntime::new(context);

        let amplitude = 1.0;
        let frequency = 1.0;
        let phase = 0.5 * PI;
        let bias = 0.0;
        let params = Parameters::new(amplitude, frequency, phase, bias);

        let mut block = TrianglewaveBlock::default();

        block.generate(&params, &runtime.context()); // T = 0
        assert_relative_eq!(block.buffer(), 0.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI / 2
        assert_relative_eq!(block.buffer(), 1.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI
        assert_relative_eq!(block.buffer(), 0.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 3PI / 2
        assert_relative_eq!(block.buffer(), -1.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 2PI
        assert_relative_eq!(block.buffer(), 0.0, epsilon = 1e-6);
    }

    #[test]
    fn test_trianglewave_block_bias() {
        let context = StubContext::new(
            Duration::from_secs(0),
            None,
            Duration::from_secs_f64(PI / 2.0),
        );
        let mut runtime = StubRuntime::new(context);

        let amplitude = 1.0;
        let frequency = 1.0;
        let phase = 0.0;
        let bias = 1.0;
        let params = Parameters::new(amplitude, frequency, phase, bias);

        let mut block = TrianglewaveBlock::default();

        block.generate(&params, &runtime.context()); // T = 0
        assert_relative_eq!(block.buffer(), 0.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI / 2
        assert_relative_eq!(block.buffer(), 1.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI
        assert_relative_eq!(block.buffer(), 2.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 3PI / 2
        assert_relative_eq!(block.buffer(), 1.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 2PI
        assert_relative_eq!(block.buffer(), 0.0, epsilon = 1e-6);
    }

    #[test]
    fn test_trianglewave_block_amplitude() {
        let context = StubContext::new(
            Duration::from_secs(0),
            None,
            Duration::from_secs_f64(PI / 2.0),
        );
        let mut runtime = StubRuntime::new(context);

        let amplitude = 2.0;
        let frequency = 1.0;
        let phase = 0.0;
        let bias = 0.0;
        let params = Parameters::new(amplitude, frequency, phase, bias);

        let mut block = TrianglewaveBlock::default();

        block.generate(&params, &runtime.context()); // T = 0
        assert_relative_eq!(block.buffer(), -2.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI / 2
        assert_relative_eq!(block.buffer(), 0.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI
        assert_relative_eq!(block.buffer(), 2.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 3PI / 2
        assert_relative_eq!(block.buffer(), 0.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 2PI
        assert_relative_eq!(block.buffer(), -2.0, epsilon = 1e-6);
    }

    #[test]
    fn test_trianglewave_block_high_time() {
        let context = StubContext::new(
            Duration::from_secs(0),
            None,
            Duration::from_secs_f64(PI / 2.0),
        );
        let mut runtime = StubRuntime::new(context);

        let amplitude = 1.0;
        let frequency = 2.0;
        let phase = 0.0;
        let bias = 0.0;

        let params = Parameters::new(amplitude, frequency, phase, bias);
        let mut block = TrianglewaveBlock::default();

        block.generate(&params, &runtime.context()); // T = 0
        assert_relative_eq!(block.buffer(), -1.0, epsilon = 1e-6);

        runtime.set_time(Duration::from_secs_f64(400.0 * PI));
        block.generate(&params, &runtime.context()); // T = 400PI
        assert_relative_eq!(block.buffer(), -1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_trianglewave_block_frequency() {
        let context = StubContext::new(
            Duration::from_secs(0),
            None,
            Duration::from_secs_f64(PI / 4.0),
        );
        let mut runtime = StubRuntime::new(context);

        let amplitude = 1.0;
        let frequency = 2.0;
        let phase = 0.0;
        let bias = 0.0;

        let params = Parameters::new(amplitude, frequency, phase, bias);
        let mut block = TrianglewaveBlock::default();

        block.generate(&params, &runtime.context()); // T = 0
        assert_relative_eq!(block.buffer(), -1.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI / 4
        assert_relative_eq!(block.buffer(), 0.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI / 2
        assert_relative_eq!(block.buffer(), 1.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 3PI / 4
        assert_relative_eq!(block.buffer(), 0.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI
        assert_relative_eq!(block.buffer(), -1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_trianglewave_block_integer_output() {
        // i16 output: computed in f64, cast with `as` semantics at the output.
        let context = StubContext::new(
            Duration::from_secs(0),
            None,
            Duration::from_secs_f64(PI / 2.0),
        );
        let mut runtime = StubRuntime::new(context);

        let params = Parameters::new(100i16, 1.0, 0.0, 25i16);
        let mut block = TrianglewaveBlock::<i16, f64>::default();

        // T = 0 is exact: (4 * 0 - 1) * 100 + 25 = -75, no truncation ambiguity.
        block.generate(&params, &runtime.context());
        assert_eq!(block.buffer(), -75);

        // T = PI / 2 is the zero crossing: the float value is within +/-1e-7 of 25 due
        // to nanosecond time rounding, and truncates to 25 from either side... almost.
        // A hair below 25.0 would truncate to 24, so allow both.
        runtime.tick();
        block.generate(&params, &runtime.context());
        assert!(block.buffer() == 25 || block.buffer() == 24);
    }

    #[test]
    fn test_trianglewave_block_integer_output_saturates() {
        // The peak can exceed the output type's range through amplitude + bias; the
        // output cast saturates rather than wrapping or panicking, so this block has
        // no arithmetic panic sites. frequency = 0 with the phase pinning the wave at
        // its peak (phase = TAU/2 -> exactly +amplitude + bias) and trough (phase = 0).
        let runtime = StubRuntime::default();

        // Peak: 100 + 100 = 200 saturates at i8::MAX.
        let params = Parameters::new(100i8, 0.0, core::f64::consts::TAU / 2.0, 100i8);
        let mut block = TrianglewaveBlock::<i8, f64>::default();
        block.generate(&params, &runtime.context());
        assert_eq!(block.buffer(), i8::MAX);

        // Trough: -100 + 100 = 0, exactly representable.
        let params = Parameters::new(100i8, 0.0, 0.0, 100i8);
        block.generate(&params, &runtime.context());
        assert_eq!(block.buffer(), 0);
    }
}
