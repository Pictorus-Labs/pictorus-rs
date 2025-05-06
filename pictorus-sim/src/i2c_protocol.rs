use core::convert::Infallible;

use pictorus_blocks::{I2cInputBlockParams, I2cOutputBlockParams};
use pictorus_traits::{ByteSliceSignal, InputBlock, OutputBlock};

pub struct SimI2cProtocol {
    buffer: Vec<u8>,
}
pub type I2cProtocolType = SimI2cProtocol;

impl SimI2cProtocol {
    pub fn new() -> Self {
        SimI2cProtocol { buffer: Vec::new() }
    }
}

impl Default for SimI2cProtocol {
    fn default() -> Self {
        Self::new()
    }
}

pub fn create_i2c_protocol() -> Result<SimI2cProtocol, Infallible> {
    Ok(SimI2cProtocol { buffer: Vec::new() })
}

impl InputBlock for SimI2cProtocol {
    type Output = ByteSliceSignal;
    type Parameters = I2cInputBlockParams;

    fn input(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
    ) -> pictorus_traits::PassBy<'_, Self::Output> {
        self.buffer.resize(parameters.read_bytes, 0);
        &self.buffer
    }
}

impl OutputBlock for SimI2cProtocol {
    type Inputs = ByteSliceSignal;
    type Parameters = I2cOutputBlockParams;

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) {
        self.buffer.clear();
        self.buffer.extend_from_slice(inputs);
    }
}
