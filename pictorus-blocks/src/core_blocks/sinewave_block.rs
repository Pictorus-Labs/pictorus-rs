use crate::traits::Float;
use block_data::BlockData;
use pictorus_traits::GeneratorBlock;

#[derive(Debug, Clone)]
/// Outputs a sinewave signal with specified amplitude, frequency, phase, and bias.
pub struct SinewaveBlock<T>
where
    T: Float,
    f64: From<T>,
{
    phantom: core::marker::PhantomData<T>,
    pub data: BlockData,
}

impl<T> Default for SinewaveBlock<T>
where
    T: Float,
    f64: From<T>,
{
    fn default() -> Self {
        Self {
            phantom: core::marker::PhantomData,
            data: BlockData::from_scalar(f64::from(T::zero())),
        }
    }
}

impl<T> GeneratorBlock for SinewaveBlock<T>
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
    ) -> pictorus_traits::PassBy<Self::Output> {
        let time = T::from_duration(context.time());
        let sin_val = parameters.amplitude
            * num_traits::Float::sin(parameters.frequency * time + parameters.phase)
            + parameters.bias;
        self.data = BlockData::from_scalar(sin_val.into());
        sin_val
    }
}

#[derive(Debug, Clone)]
pub struct Parameters<T: Float> {
    pub amplitude: T,
    pub frequency: T,
    pub phase: T,
    pub bias: T,
}

impl<T: Float> Parameters<T> {
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

    use crate::testing::StubContext;
    use core::time::Duration;
    use num_traits::Float;

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
        assert_eq!(block.data.scalar(), Float::sin(0.5));
        context.time = Duration::from_secs(1);

        assert_eq!(block.generate(&parameters, &context), Float::sin(1.5));
        assert_eq!(block.data.scalar(), Float::sin(1.5));
    }
}
