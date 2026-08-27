use super::cast_block::CastElement;
use crate::traits::{Float, Scalar};
use pictorus_traits::{GeneratorBlock, PassBy};

#[derive(Debug, Clone)]
/// Outputs a signal that ramps up linearly from a specified start time at a specified rate.
///
/// The ramp is computed in the float type `F` (rate and start_time are float so
/// fractional rates work for any output type) and cast to the output type `T` with `as`
/// semantics.
pub struct RampBlock<T, F = T>
where
    T: Scalar,
    F: Float + CastElement<T>,
    f64: From<F>,
{
    phantom: core::marker::PhantomData<F>,
    buffer: T,
}

impl<T, F> Default for RampBlock<T, F>
where
    T: Scalar,
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

impl<T, F> GeneratorBlock for RampBlock<T, F>
where
    T: Scalar,
    F: Float + CastElement<T>,
    f64: From<F>,
{
    type Parameters = Parameters<F>;
    type Output = T;

    fn generate(
        &mut self,
        parameters: &Self::Parameters,
        context: &dyn pictorus_traits::Context,
    ) -> pictorus_traits::PassBy<'_, Self::Output> {
        let time = F::from_duration(context.time());
        let ramp_val =
            parameters.rate * num_traits::Float::max(time - parameters.start_time, F::zero());
        self.buffer = ramp_val.cast_element();
        self.buffer
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        self.buffer
    }
}

#[derive(Debug)]
pub struct Parameters<F: Float> {
    pub start_time: F,
    pub rate: F,
}

impl<F: Float> Parameters<F> {
    pub fn new(start_time: F, rate: F) -> Self {
        Self { start_time, rate }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::testing::{StubContext, StubRuntime};
    use core::time::Duration;

    #[test]
    fn test_ramp_default_buffer_no_panic() {
        let block = RampBlock::<f64>::default();
        assert_eq!(block.buffer(), 0.0);
    }

    #[test]
    fn test_ramp_block() {
        let mut block = RampBlock::<f64>::default();
        let mut runtime = StubRuntime::new(StubContext::new(
            Duration::from_secs_f64(0.0),
            None,
            Duration::from_secs_f64(1.0),
        ));

        // Slope is 1.0, start time is 0.0
        let parameters = Parameters::new(0.0, 1.0);
        let output = block.generate(&parameters, &runtime.context());
        assert_eq!(output, 0.0);
        assert_eq!(block.buffer(), output);

        runtime.tick();
        let output = block.generate(&parameters, &runtime.context());
        assert_eq!(output, 1.0);

        runtime.tick();
        let output = block.generate(&parameters, &runtime.context());
        assert_eq!(output, 2.0);

        // Slope is 3.0, start time is 1.0
        runtime.context.time = Duration::from_secs_f64(0.0); //reset time
        let parameters = Parameters::new(1.0, 3.0);
        let output = block.generate(&parameters, &runtime.context());
        assert_eq!(output, 0.0);

        runtime.tick();
        let output = block.generate(&parameters, &runtime.context());
        assert_eq!(output, 0.0);

        runtime.tick();
        let output = block.generate(&parameters, &runtime.context());
        assert_eq!(output, 3.0);

        runtime.tick();
        let output = block.generate(&parameters, &runtime.context());
        assert_eq!(output, 6.0);
    }

    #[test]
    fn test_ramp_block_integer_output() {
        // u16 output with a fractional rate: computed in f64, truncated by the output
        // cast — a staircase that steps up every 2 seconds.
        let mut block = RampBlock::<u16, f64>::default();
        let mut runtime = StubRuntime::new(StubContext::new(
            Duration::from_secs_f64(0.0),
            None,
            Duration::from_secs_f64(1.0),
        ));

        let parameters = Parameters::new(0.0, 0.5);
        assert_eq!(block.generate(&parameters, &runtime.context()), 0);

        runtime.tick(); // t = 1.0 -> 0.5 truncates to 0
        assert_eq!(block.generate(&parameters, &runtime.context()), 0);

        runtime.tick(); // t = 2.0 -> 1
        assert_eq!(block.generate(&parameters, &runtime.context()), 1);

        runtime.tick(); // t = 3.0 -> 1.5 truncates to 1
        assert_eq!(block.generate(&parameters, &runtime.context()), 1);

        runtime.tick(); // t = 4.0 -> 2
        assert_eq!(block.generate(&parameters, &runtime.context()), 2);
    }

    #[test]
    fn test_ramp_block_integer_output_saturates() {
        // Once the float ramp value exceeds the output type's range, the output cast
        // saturates at the type's max rather than wrapping or panicking, so this block
        // has no arithmetic panic sites. A ramp past an integer type's ceiling just
        // flatlines there.
        let mut block = RampBlock::<u8, f64>::default();
        let mut runtime = StubRuntime::new(StubContext::new(
            Duration::from_secs_f64(0.0),
            None,
            Duration::from_secs_f64(1.0),
        ));

        let parameters = Parameters::new(0.0, 1000.0);
        runtime.tick(); // t = 1.0 -> 1000 saturates at 255
        assert_eq!(block.generate(&parameters, &runtime.context()), u8::MAX);
    }
}
