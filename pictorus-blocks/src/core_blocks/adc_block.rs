use block_data::BlockData as OldBlockData;
use num_traits::Float;
use pictorus_traits::{Context, PassBy, ProcessBlock, Scalar};

/// Parameters for the ADC block
#[doc(hidden)]
pub struct Parameters;

impl Default for Parameters {
    fn default() -> Self {
        Self::new()
    }
}

impl Parameters {
    pub fn new() -> Parameters {
        Parameters {}
    }
}

/// Store data received from the ADC.
///
/// This block ensures that the ADC data is cached and the same for all blocks in a state
/// for a given tick.
///
/// Each platform will need to implement an `InputBlock` on the ADC hardware
/// and pass those results into this block.
pub struct AdcBlock<I: Scalar, O: Float> {
    pub data: OldBlockData,
    buffer: O,
    phantom: core::marker::PhantomData<I>,
}

impl<I, O> Default for AdcBlock<I, O>
where
    I: Scalar,
    O: Float + num_traits::Zero,
{
    fn default() -> Self {
        AdcBlock {
            data: OldBlockData::from_scalar(0.0),
            buffer: O::zero(),
            phantom: core::marker::PhantomData,
        }
    }
}

impl<I, O> ProcessBlock for AdcBlock<I, O>
where
    I: Scalar + num_traits::ToPrimitive,
    O: Float + pictorus_traits::Scalar,
{
    type Parameters = Parameters;
    type Inputs = I;
    type Output = O;

    fn process<'b>(
        &'b mut self,
        _parameters: &Self::Parameters,
        _context: &dyn Context,
        input: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        self.buffer = O::from(input).expect("Failed to convert input to output");
        self.data = OldBlockData::from_scalar(self.buffer.into());
        self.buffer
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::testing::StubContext;

    #[test]
    fn test_adc_block_f64() {
        let c = StubContext::default();
        let mut block = AdcBlock::<u16, f64>::default();
        let input = 42u16;
        let output = block.process(&Parameters::new(), &c, input);
        assert_eq!(output, 42.0);
    }

    #[test]
    fn test_adc_block_f32() {
        let c = StubContext::default();
        let mut block = AdcBlock::<u16, f32>::default();
        let input = 42u16;
        let output = block.process(&Parameters::new(), &c, input);
        assert_eq!(output, 42.0);
    }
}
