use crate::traits::Float;
use pictorus_block_data::BlockData;
use pictorus_traits::GeneratorBlock;

pub struct Parameters<T: Float> {
    pub amplitude: T,
    pub on_duration: T,
    pub off_duration: T,
    pub phase: T,
    pub bias: T,
}

impl<T: Float> Parameters<T> {
    pub fn new(amplitude: T, on_duration: T, off_duration: T, phase: T, bias: T) -> Self {
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
pub struct SquarewaveBlock<T: Float> {
    phantom_output_type: core::marker::PhantomData<T>,
    pub data: BlockData,
}

impl<T: Float> Default for SquarewaveBlock<T>
where
    f64: From<T>,
{
    fn default() -> Self {
        Self {
            phantom_output_type: core::marker::PhantomData,
            data: BlockData::from_scalar(T::zero().into()),
        }
    }
}

impl<T> GeneratorBlock for SquarewaveBlock<T>
where
    T: Float,
    f64: From<T>,
{
    type Output = T;
    type Parameters = Parameters<T>;

    fn generate(
        &mut self,
        parameters: &Self::Parameters,
        context: &dyn pictorus_traits::Context,
    ) -> pictorus_traits::PassBy<'_, Self::Output> {
        let adjusted_time = Self::Output::from_duration(context.time()) - parameters.phase;
        let pulse_time = parameters.on_duration + parameters.off_duration;
        let mut time_since_last_pulse_start: Self::Output = adjusted_time % pulse_time;

        if time_since_last_pulse_start < T::zero() {
            // Adjust for negative phase
            time_since_last_pulse_start += pulse_time
        };

        let output = if time_since_last_pulse_start > parameters.on_duration {
            parameters.bias
        } else {
            parameters.bias + parameters.amplitude
        };
        self.data = BlockData::from_scalar(f64::from(output));
        output
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::StubRuntime;

    use super::*;
    use core::time::Duration;

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

        block.generate(&p, &runtime.context());
        assert_eq!(block.generate(&p, &runtime.context()), bias);
        assert_eq!(block.data.scalar(), bias);

        runtime.set_time(Duration::from_millis(500));
        assert_eq!(block.generate(&p, &runtime.context()), bias + amplitude);
        assert_eq!(block.data.scalar(), bias + amplitude);

        runtime.set_time(Duration::from_secs_f64(1.0));
        assert_eq!(block.generate(&p, &runtime.context()), bias + amplitude);
        assert_eq!(block.data.scalar(), bias + amplitude);

        runtime.set_time(Duration::from_secs_f64(1.499));
        assert_eq!(block.generate(&p, &runtime.context()), bias + amplitude);
        assert_eq!(block.data.scalar(), bias + amplitude);

        runtime.set_time(Duration::from_secs_f64(1.5));
        assert_eq!(block.generate(&p, &runtime.context()), bias + amplitude);
        assert_eq!(block.data.scalar(), bias + amplitude);

        // Off duration
        runtime.set_time(Duration::from_secs_f64(2.5));
        assert_eq!(block.generate(&p, &runtime.context()), bias);
        assert_eq!(block.data.scalar(), bias);

        runtime.set_time(Duration::from_secs_f64(3.499));
        assert_eq!(block.generate(&p, &runtime.context()), bias);
        assert_eq!(block.data.scalar(), bias);

        // Back on
        runtime.set_time(Duration::from_secs_f64(3.5));
        assert_eq!(block.generate(&p, &runtime.context()), bias + amplitude);
        assert_eq!(block.data.scalar(), bias + amplitude);
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

        block.generate(&p, &runtime.context());
        assert_eq!(block.generate(&p, &runtime.context()), bias);
        assert_eq!(block.data.scalar() as f32, bias);

        runtime.set_time(Duration::from_millis(500));
        assert_eq!(block.generate(&p, &runtime.context()), bias + amplitude);
        assert_eq!(block.data.scalar() as f32, bias + amplitude);

        runtime.set_time(Duration::from_secs_f32(1.0));
        assert_eq!(block.generate(&p, &runtime.context()), bias + amplitude);
        assert_eq!(block.data.scalar() as f32, bias + amplitude);

        runtime.set_time(Duration::from_secs_f32(1.499));
        assert_eq!(block.generate(&p, &runtime.context()), bias + amplitude);
        assert_eq!(block.data.scalar() as f32, bias + amplitude);

        runtime.set_time(Duration::from_secs_f32(1.5));
        assert_eq!(block.generate(&p, &runtime.context()), bias + amplitude);
        assert_eq!(block.data.scalar() as f32, bias + amplitude);

        // Off duration
        runtime.set_time(Duration::from_secs_f32(2.5));
        assert_eq!(block.generate(&p, &runtime.context()), bias);
        assert_eq!(block.data.scalar() as f32, bias);

        runtime.set_time(Duration::from_secs_f32(3.499));
        assert_eq!(block.generate(&p, &runtime.context()), bias);
        assert_eq!(block.data.scalar() as f32, bias);

        // Back on
        runtime.set_time(Duration::from_secs_f32(3.5));
        assert_eq!(block.generate(&p, &runtime.context()), bias + amplitude);
        assert_eq!(block.data.scalar() as f32, bias + amplitude);
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
        block.generate(&p, &runtime.context());
        assert_eq!(block.generate(&p, &runtime.context()), bias);

        p.phase = 0.0;
        assert_eq!(block.generate(&p, &runtime.context()), bias + amplitude);

        p.phase = 0.5;
        assert_eq!(block.generate(&p, &runtime.context()), bias);

        p.phase = 2.5;
        assert_eq!(block.generate(&p, &runtime.context()), bias + amplitude);

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

        let output = block.generate(&p, &runtime.context());
        assert_eq!(output, bias + amplitude);

        runtime.set_time(Duration::from_secs_f32(1.5));
        let output = block.generate(&p, &runtime.context());
        assert_eq!(output, bias);
    }
}
