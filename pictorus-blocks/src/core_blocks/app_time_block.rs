use super::cast_block::CastElement;
use crate::traits::{Float, Scalar};
use pictorus_traits::{GeneratorBlock, PassBy};

#[derive(Debug, Clone, Default)]
pub struct Parameters {}

impl Parameters {
    pub fn new() -> Self {
        Self {}
    }
}

/// Outputs the elapsed application time as a scalar value.
///
/// The time is read in the float type `F` and cast to the output type `T` with `as`
/// semantics (integer outputs are truncated seconds).
#[derive(Debug, Clone)]
pub struct AppTimeBlock<T, F = T>
where
    T: Scalar,
    F: Float + CastElement<T>,
    f64: From<F>,
{
    phantom: core::marker::PhantomData<F>,
    buffer: T,
}

impl<T, F> Default for AppTimeBlock<T, F>
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

impl<T, F> GeneratorBlock for AppTimeBlock<T, F>
where
    T: Scalar,
    F: Float + CastElement<T>,
    f64: From<F>,
{
    type Parameters = Parameters;
    type Output = T;

    fn generate(
        &mut self,
        _parameters: &Self::Parameters,
        context: &dyn pictorus_traits::Context,
    ) -> pictorus_traits::PassBy<'_, Self::Output> {
        let time = F::from_duration(context.time());
        self.buffer = time.cast_element();
        self.buffer
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        self.buffer
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::StubRuntime;
    use crate::AppTimeBlock;
    use core::time::Duration;
    use pictorus_traits::GeneratorBlock;

    #[test]
    fn test_app_time_default_buffer_no_panic() {
        let block = AppTimeBlock::<f64>::default();
        assert_eq!(block.buffer(), 0.0);
    }

    #[test]
    fn test_app_time_block() {
        let mut runtime = StubRuntime::default();

        let mut block = AppTimeBlock::<f64>::default();
        let parameters = <AppTimeBlock<f64> as GeneratorBlock>::Parameters::new();

        for _ in 0..100 {
            let context = runtime.context();
            let output = block.generate(&parameters, &context);
            assert_eq!(output, block.buffer());
            runtime.tick();
        }
    }

    #[test]
    fn test_app_time_block_integer_output() {
        let mut runtime = StubRuntime::default();

        // u32 output: whole seconds, truncated.
        let mut block = AppTimeBlock::<u32, f64>::default();
        let parameters = <AppTimeBlock<u32, f64> as GeneratorBlock>::Parameters::new();

        runtime.set_time(Duration::from_millis(2500));
        assert_eq!(block.generate(&parameters, &runtime.context()), 2);
    }

    #[test]
    fn test_app_time_block_integer_output_saturates() {
        // Past the output type's range the cast saturates rather than wrapping or
        // panicking (no arithmetic panic sites): a u8 app time pegs at 255 seconds
        // forever.
        let mut runtime = StubRuntime::default();

        let mut block = AppTimeBlock::<u8, f64>::default();
        let parameters = <AppTimeBlock<u8, f64> as GeneratorBlock>::Parameters::new();

        runtime.set_time(Duration::from_secs(300));
        assert_eq!(block.generate(&parameters, &runtime.context()), u8::MAX);
    }
}
