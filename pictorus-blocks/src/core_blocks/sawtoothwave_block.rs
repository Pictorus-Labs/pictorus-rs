use super::cast_block::CastElement;
use crate::traits::{Float, Scalar};
use pictorus_traits::{GeneratorBlock, PassBy};

#[derive(Debug, Clone)]
/// Outputs a sawtooth wave signal with specified amplitude, frequency, phase, and bias.
///
/// The wave is computed in the float type `F` and cast to the output type `T` with
/// `as` semantics. Value-domain parameters (amplitude, bias) are specified in `T`;
/// time-domain parameters (frequency, phase) in `F`.
pub struct SawtoothwaveBlock<T, F = T>
where
    T: Scalar + CastElement<F>,
    F: Float + CastElement<T>,
    f64: From<F>,
{
    phantom: core::marker::PhantomData<F>,
    buffer: T,
}

impl<T, F> Default for SawtoothwaveBlock<T, F>
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

impl<T, F> GeneratorBlock for SawtoothwaveBlock<T, F>
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
        let two = F::one() + F::one();
        let amplitude: F = parameters.amplitude.cast_element();
        let bias: F = parameters.bias.cast_element();
        let time =
            (parameters.frequency * F::from_duration(context.time()) + parameters.phase) / F::TAU;
        let x = two * (time - num_traits::Float::floor(time)) - F::one();
        let val = amplitude * x + bias;
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

impl<T, F> Parameters<T, F>
where
    T: Scalar,
    F: Float,
{
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
    fn test_sawtoothwave_default_buffer_no_panic() {
        let block = SawtoothwaveBlock::<f64>::default();
        assert_eq!(block.buffer(), 0.0);
    }

    #[test]
    fn test_sawtoothwave_block_simple() {
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

        let mut block = SawtoothwaveBlock::default();

        let out = block.generate(&params, &runtime.context()); // T = 0
        assert_relative_eq!(block.buffer(), -1.0, epsilon = 0.00001);
        assert_eq!(block.buffer(), out);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI / 2
        assert_relative_eq!(block.buffer(), -0.5, epsilon = 0.00001);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI
        assert_relative_eq!(block.buffer(), 0.0, epsilon = 0.00001);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 3 * PI / 2
        assert_relative_eq!(block.buffer(), 0.5, epsilon = 0.00001);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 2 * PI
        assert_relative_eq!(block.buffer(), -1.0, epsilon = 0.00001);
    }

    #[test]
    fn test_sawtoothwave_block_phase() {
        let context = StubContext::new(
            Duration::from_secs(0),
            None,
            Duration::from_secs_f64(PI / 2.0),
        );
        let mut runtime = StubRuntime::new(context);

        let amplitude = 1.0;
        let frequency = 1.0;
        let phase = PI;
        let bias = 0.0;
        let params = Parameters::new(amplitude, frequency, phase, bias);

        let mut block = SawtoothwaveBlock::default();

        block.generate(&params, &runtime.context()); // T = 0
        assert_relative_eq!(block.buffer(), 0.0, epsilon = 0.00001);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI / 2
        assert_relative_eq!(block.buffer(), 0.5, epsilon = 0.00001);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI
        assert_relative_eq!(block.buffer(), -1.0, epsilon = 0.00001);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 3 * PI / 2
        assert_relative_eq!(block.buffer(), -0.5, epsilon = 0.00001);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 2 * PI
        assert_relative_eq!(block.buffer(), 0.0, epsilon = 0.00001);
    }

    #[test]
    fn test_sawtoothwave_block_bias() {
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

        let mut block = SawtoothwaveBlock::default();

        block.generate(&params, &runtime.context()); // T = 0
        assert_relative_eq!(block.buffer(), 0.0, epsilon = 0.00001);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI / 2
        assert_relative_eq!(block.buffer(), 0.5, epsilon = 0.00001);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI
        assert_relative_eq!(block.buffer(), 1.0, epsilon = 0.00001);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 3 * PI / 2
        assert_relative_eq!(block.buffer(), 1.5, epsilon = 0.00001);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 2 * PI
        assert_relative_eq!(block.buffer(), 0.0, epsilon = 0.00001);
    }

    #[test]
    fn test_sawtoothwave_block_amplitude() {
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

        let mut block = SawtoothwaveBlock::default();

        block.generate(&params, &runtime.context()); // T = 0
        assert_relative_eq!(block.buffer(), -2.0, epsilon = 0.00001);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI / 2
        assert_relative_eq!(block.buffer(), -1.0, epsilon = 0.00001);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI
        assert_relative_eq!(block.buffer(), 0.0, epsilon = 0.00001);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 3 * PI / 2
        assert_relative_eq!(block.buffer(), 1.0, epsilon = 0.00001);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 2 * PI
        assert_relative_eq!(block.buffer(), -2.0, epsilon = 0.00001);
    }

    #[test]
    fn test_sawtoothwave_block_high_time() {
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

        let mut block = SawtoothwaveBlock::default();

        block.generate(&params, &runtime.context()); // T = 0
        assert_relative_eq!(block.buffer(), -1.0, epsilon = 0.00001);

        // This was a little weird it was just shy of hitting the discontinuity at 2PI so it was just barely less than 1.0,
        // this fudge factor pushes it over the line. Testing on the edge of the discontinuity might not be the best approach
        runtime.context.time = Duration::from_secs_f64(2.0 * PI + 0.0000001);
        block.generate(&params, &runtime.context()); // T = 2*PI
        assert_relative_eq!(block.buffer(), -1.0, epsilon = 0.00001);

        runtime.context.time = Duration::from_secs_f64(400.0 * PI);
        block.generate(&params, &runtime.context()); // T = 400 * PI
        assert_relative_eq!(block.buffer(), -1.0, epsilon = 0.00001);

        runtime.context.time = Duration::from_secs_f64(400.5 * PI);
        block.generate(&params, &runtime.context()); // T = 400.5 * PI
        assert_relative_eq!(block.buffer(), -0.5, epsilon = 0.00001);
    }

    #[test]
    fn test_sawtoothwave_block_frequency() {
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

        let mut block = SawtoothwaveBlock::default();

        block.generate(&params, &runtime.context()); // T = 0
        assert_relative_eq!(block.buffer(), -1.0, epsilon = 0.00001);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI / 2
        assert_relative_eq!(block.buffer(), -0.5, epsilon = 0.00001);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI
        assert_relative_eq!(block.buffer(), 0.0, epsilon = 0.00001);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 3 * PI / 2
        assert_relative_eq!(block.buffer(), 0.5, epsilon = 0.00001);
    }

    #[test]
    fn test_sawtoothwave_block_integer_output() {
        // i32 output: computed in f64, cast with `as` semantics at the output. T = 0 is
        // exact (x = -1), so the trough value has no truncation ambiguity.
        let context = StubContext::new(
            Duration::from_secs(0),
            None,
            Duration::from_secs_f64(PI / 2.0),
        );
        let runtime = StubRuntime::new(context);

        let params = Parameters::new(1000i32, 1.0, 0.0, 250i32);
        let mut block = SawtoothwaveBlock::<i32, f64>::default();

        block.generate(&params, &runtime.context()); // T = 0
        assert_eq!(block.buffer(), -750);
    }

    #[test]
    fn test_sawtoothwave_block_integer_output_saturates() {
        // The wave's value can exceed the output type's range through amplitude and
        // bias; the output cast saturates rather than wrapping or panicking, so this
        // block has no arithmetic panic sites. The trough at T = 0 is exactly
        // -amplitude + bias: -100 + -100 = -200 saturates at i8::MIN.
        let runtime = StubRuntime::default();

        let params = Parameters::new(100i8, 1.0, 0.0, -100i8);
        let mut block = SawtoothwaveBlock::<i8, f64>::default();

        block.generate(&params, &runtime.context());
        assert_eq!(block.buffer(), i8::MIN);
    }
}
