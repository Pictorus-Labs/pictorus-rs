use crate::traits::Float;
use block_data::BlockData;
use pictorus_traits::GeneratorBlock;

#[derive(Debug, Clone)]
/// Outputs a triangle wave signal with specified amplitude, frequency, phase, and bias.
pub struct TrianglewaveBlock<T>
where
    T: Float,
    f64: From<T>,
{
    phantom: core::marker::PhantomData<T>,
    pub data: BlockData,
}

impl<T> Default for TrianglewaveBlock<T>
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

impl<T> GeneratorBlock for TrianglewaveBlock<T>
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
        // These two variables are used to construct constants used in the math below in a way that is infallible and generic
        let two: T = T::one() + T::one();
        let four: T = two + two;
        let t =
            (parameters.frequency * T::from_duration(context.time()) + parameters.phase) / (T::TAU);
        let t = num_traits::Float::fract(t);
        let y = if t < T::one() / two { t } else { T::one() - t };
        // y is in the range [0, 0.5] over a t value from 0 to 1. Scale it by 4 ( to a range of [0, 2] )
        // then shift it down by 1 to get it in the range [-1, 1], then scale it by the amplitude and add the bias.
        let val = (four * y - T::one()) * parameters.amplitude + parameters.bias;
        self.data = BlockData::from_scalar(val.into());
        val
    }
}

pub struct Parameters<T: Float> {
    pub amplitude: T,
    pub frequency: T,
    pub phase: T,
    pub bias: T,
}

impl<T: Float> Parameters<T> {
    pub fn new(amplitude: T, frequency: T, phase: T, bias: T) -> Parameters<T> {
        Parameters {
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

        block.generate(&params, &runtime.context()); // T = 0
        assert_relative_eq!(block.data.scalar(), -1.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI / 2
        assert_relative_eq!(block.data.scalar(), 0.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI
        assert_relative_eq!(block.data.scalar(), 1.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 3PI / 2
        assert_relative_eq!(block.data.scalar(), 0.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 2PI
        assert_relative_eq!(block.data.scalar(), -1.0, epsilon = 1e-6);
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
        assert_relative_eq!(block.data.scalar(), 0.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI / 2
        assert_relative_eq!(block.data.scalar(), 1.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI
        assert_relative_eq!(block.data.scalar(), 0.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 3PI / 2
        assert_relative_eq!(block.data.scalar(), -1.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 2PI
        assert_relative_eq!(block.data.scalar(), 0.0, epsilon = 1e-6);
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
        assert_relative_eq!(block.data.scalar(), 0.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI / 2
        assert_relative_eq!(block.data.scalar(), 1.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI
        assert_relative_eq!(block.data.scalar(), 2.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 3PI / 2
        assert_relative_eq!(block.data.scalar(), 1.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 2PI
        assert_relative_eq!(block.data.scalar(), 0.0, epsilon = 1e-6);
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
        assert_relative_eq!(block.data.scalar(), -2.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI / 2
        assert_relative_eq!(block.data.scalar(), 0.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI
        assert_relative_eq!(block.data.scalar(), 2.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 3PI / 2
        assert_relative_eq!(block.data.scalar(), 0.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 2PI
        assert_relative_eq!(block.data.scalar(), -2.0, epsilon = 1e-6);
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
        assert_relative_eq!(block.data.scalar(), -1.0, epsilon = 1e-6);

        runtime.set_time(Duration::from_secs_f64(400.0 * PI));
        block.generate(&params, &runtime.context()); // T = 400PI
        assert_relative_eq!(block.data.scalar(), -1.0, epsilon = 1e-6);
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
        assert_relative_eq!(block.data.scalar(), -1.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI / 4
        assert_relative_eq!(block.data.scalar(), 0.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI / 2
        assert_relative_eq!(block.data.scalar(), 1.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = 3PI / 4
        assert_relative_eq!(block.data.scalar(), 0.0, epsilon = 1e-6);

        runtime.tick();
        block.generate(&params, &runtime.context()); // T = PI
        assert_relative_eq!(block.data.scalar(), -1.0, epsilon = 1e-6);
    }
}
