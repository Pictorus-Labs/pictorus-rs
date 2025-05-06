use crate::traits::Scalar;
use pictorus_traits::{Context, Pass, PassBy, ProcessBlock};

/// Parameters for the PWM block
pub struct Parameters {}

impl Default for Parameters {
    fn default() -> Self {
        Self::new()
    }
}

impl Parameters {
    pub fn new() -> Self {
        Self {}
    }
}

/// A block buffers frequency and duty cycle to a PWM peripheral as a tuple
/// (frequency, duty cycle). Duty cycle is a value between 0 and 1 and
/// represents the percentage of time the signal is high in a PWM cycle.
///
/// This block automatically clamps the duty cycle to the range [0, 1].
pub struct PwmBlock<T: Default + Scalar, I: Pass> {
    pwm_values: Option<I>,
    _phantom: core::marker::PhantomData<T>,
}

impl<T, I> Default for PwmBlock<T, I>
where
    T: Default + Scalar,
    I: Pass,
{
    fn default() -> Self {
        PwmBlock {
            pwm_values: None,
            _phantom: core::marker::PhantomData,
        }
    }
}

impl<T> ProcessBlock for PwmBlock<T, (T, T)>
where
    T: Default + Scalar + num_traits::Zero + num_traits::One + num_traits::Float,
{
    type Inputs = (T, T); // (Frequency, Duty Cycle)
    type Output = (T, T); // (Frequency, Duty Cycle)
    type Parameters = Parameters;

    fn process<'b>(
        &'b mut self,
        _parameters: &Self::Parameters,
        _context: &dyn Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        let (frequency, duty_cycle) = inputs;
        let duty_cycle_clamped = duty_cycle.clamp(T::zero(), T::one());
        let frequency_clamped = frequency.clamp(T::zero(), T::max_value());
        let output = self
            .pwm_values
            .insert((frequency_clamped, duty_cycle_clamped));
        *output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;

    #[test]
    fn test_pwm_block() {
        let mut block = PwmBlock::<f32, (f32, f32)>::default();
        let context = StubContext::default();

        let inputs = (1000.0, 0.5);
        let output = block.process(&Parameters::new(), &context, inputs);
        assert_eq!(output, (1000.0, 0.5));

        let inputs = (2000.0, 1.5);
        let output = block.process(&Parameters::new(), &context, inputs);
        assert_eq!(output, (2000.0, 1.0)); // Duty cycle clamped to 1.0

        let inputs = (3000.0, -0.5);
        let output = block.process(&Parameters::new(), &context, inputs);
        assert_eq!(output, (3000.0, 0.0)); // Duty cycle clamped to 0.0
    }
}
