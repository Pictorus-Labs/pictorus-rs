use pictorus_traits::{ModelClock, Matrix, Pass, PassBy, ProcessBlock};

use crate::traits::Float;

/// Parameters for Dac Block
#[doc(hidden)]
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

/// Buffer data to be sent to a DAC (Digital-to-Analog Converter).
pub struct DacBlock<O: Pass> {
    buffer: O,
}

impl<O> Default for DacBlock<O>
where
    O: Pass + Default,
{
    fn default() -> Self {
        DacBlock {
            buffer: O::default(),
        }
    }
}

impl<F> ProcessBlock for DacBlock<Matrix<1, 2, F>>
where
    F: Float,
{
    type Parameters = Parameters;
    type Inputs = Matrix<1, 2, F>;
    type Output = Matrix<1, 2, F>;

    fn process<'b>(
        &'b mut self,
        _parameters: &Self::Parameters,
        _model_clock: &dyn ModelClock,
        input: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        self.buffer = *input;
        &self.buffer
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        self.buffer.as_by()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubModelClock;

    #[test]
    fn test_dac_default_buffer_no_panic() {
        let block = DacBlock::<Matrix<1, 2, f64>>::default();
        assert_eq!(block.buffer(), &Matrix::<1, 2, f64>::zeroed());
    }

    #[test]
    fn test_dac_block() {
        let mut dac_block = DacBlock::<Matrix<1, 2, f64>>::default();
        let model_clock = StubModelClock::default();
        let output =
            *dac_block.process(&Parameters::new(), &model_clock, &Matrix { data: [[1.], [2.]] });
        assert_eq!(output.data, [[1.], [2.]]);
        assert_eq!(dac_block.buffer(), &output);
    }
}
