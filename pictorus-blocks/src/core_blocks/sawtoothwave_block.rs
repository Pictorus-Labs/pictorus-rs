use crate::traits::Float;
use pictorus_traits::GeneratorBlock;

#[derive(Debug, Clone)]
/// Outputs a sawtooth wave signal with specified amplitude, frequency, phase, and bias.
pub struct SawtoothwaveBlock<T>
where
    T: Float,
    f64: From<T>,
{
    phantom: core::marker::PhantomData<T>,
}

impl<T> Default for SawtoothwaveBlock<T>
where
    T: Float,
    f64: From<T>,
{
    fn default() -> Self {
        Self {
            phantom: core::marker::PhantomData,
        }
    }
}

impl<T> GeneratorBlock for SawtoothwaveBlock<T>
where
    T: Float,
    f64: From<T>,
{
    type Parameters = Parameters<T>;
    type Output = T;

    fn generate(
        &mut self,
        parameters: &Self::Parameters,
        context: &dyn pictorus_traits::Context,
    ) -> pictorus_traits::PassBy<'_, Self::Output> {
        let two = T::one() + T::one();
        let time =
            (parameters.frequency * T::from_duration(context.time()) + parameters.phase) / T::TAU;
        let x = two * (time - num_traits::Float::floor(time)) - T::one();
        
        parameters.amplitude * x + parameters.bias
    }
}

pub struct Parameters<T: Float> {
    pub amplitude: T,
    pub frequency: T,
    pub phase: T,
    pub bias: T,
}

impl<T> Parameters<T>
where
    T: Float,
{
    pub fn new(amplitude: T, frequency: T, phase: T, bias: T) -> Self {
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
    use core::time::Duration;

    const PI: f64 = core::f64::consts::PI;

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

        block.generate(&params, &runtime.context()); // T = 0

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI / 2

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 3 * PI / 2

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 2 * PI
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

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI / 2

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 3 * PI / 2

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 2 * PI
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

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI / 2

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 3 * PI / 2

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 2 * PI
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

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI / 2

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 3 * PI / 2

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 2 * PI
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

        // This was a little weird it was just shy of hitting the discontinuity at 2PI so it was just barely less than 1.0,
        // this fudge factor pushes it over the line. Testing on the edge of the discontinuity might not be the best approach
        runtime.context.time = Duration::from_secs_f64(2.0 * PI + 0.0000001);
        block.generate(&params, &runtime.context()); // T = 2*PI

        runtime.context.time = Duration::from_secs_f64(400.0 * PI);
        block.generate(&params, &runtime.context()); // T = 400 * PI

        runtime.context.time = Duration::from_secs_f64(400.5 * PI);
        block.generate(&params, &runtime.context()); // T = 400.5 * PI
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

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI / 2

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 3 * PI / 2
    }
}
