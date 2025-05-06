use std::convert::Infallible;

use pictorus_blocks::{SpiReceiveBlockParams, SpiTransmitBlockParams};
use pictorus_traits::{ByteSliceSignal, Context, InputBlock, OutputBlock, PassBy};

use pictorus_internal::protocols::Flush;

pub struct SimSpi {
    cache: Vec<u8>,
}

impl SimSpi {
    pub fn new() -> Result<Self, Infallible> {
        Ok(SimSpi { cache: Vec::new() })
    }
}

impl InputBlock for SimSpi {
    type Output = ByteSliceSignal;
    type Parameters = SpiReceiveBlockParams;

    fn input(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn Context,
    ) -> PassBy<'_, Self::Output> {
        self.cache.resize(parameters.read_bytes, 0);
        &self.cache
    }
}

impl OutputBlock for SimSpi {
    type Inputs = ByteSliceSignal;
    type Parameters = SpiTransmitBlockParams;

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn Context,
        _inputs: PassBy<'_, Self::Inputs>,
    ) {
        // Do nothing
        // This is a simulation, so we don't actually send any data
    }
}

impl Flush for SimSpi {
    fn flush(&mut self) {}
}
