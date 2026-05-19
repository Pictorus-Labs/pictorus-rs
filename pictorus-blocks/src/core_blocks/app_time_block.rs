use crate::traits::Float;
use pictorus_traits::{GeneratorBlock, PassBy, Scalar};

#[derive(Debug, Clone, Default)]
pub struct Parameters {}

impl Parameters {
    pub fn new() -> Self {
        Self {}
    }
}

/// Outputs the elapsed application time as a scalar value.
#[derive(Debug, Clone)]
pub struct AppTimeBlock<T: Scalar + Float> {
    phantom: core::marker::PhantomData<T>,
    buffer: T,
}

impl<T: Scalar + Float> Default for AppTimeBlock<T>
where
    f64: From<T>,
{
    fn default() -> Self {
        Self {
            phantom: core::marker::PhantomData,
            buffer: T::zero(),
        }
    }
}

impl<T> GeneratorBlock for AppTimeBlock<T>
where
    T: Scalar + Float,
    f64: From<T>,
{
    type Parameters = Parameters;
    type Output = T;

    fn generate(
        &mut self,
        _parameters: &Self::Parameters,
        context: &dyn pictorus_traits::Context,
    ) -> pictorus_traits::PassBy<'_, Self::Output> {
        let time = T::from_duration(context.time());
        self.buffer = time;
        time
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        self.buffer
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::StubRuntime;
    use crate::AppTimeBlock;
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
}
