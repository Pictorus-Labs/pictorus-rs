extern crate alloc;
use alloc::vec::Vec;
use block_data::BlockData as OldBlockData;
use pictorus_traits::{ByteSliceSignal, Context, PassBy, ProcessBlock};

/// Parameters for I2C Output Block
#[doc(hidden)]
pub struct Parameters {
    /// 8-bit address to write to
    pub address: u8,
    /// 8-bit command to send, typically a register address
    pub command: u8,
}

impl Parameters {
    pub fn new(address: f64, command: f64) -> Self {
        let addr_u8 = address as u8;
        let command_u8 = command as u8;

        Self {
            address: addr_u8,
            command: command_u8,
        }
    }
}

/// I2C Output Block buffers data to write to an I2C peripheral.
pub struct I2cOutputBlock {
    pub data: OldBlockData,
    buffer: Vec<u8>,
}

impl Default for I2cOutputBlock {
    fn default() -> Self {
        Self {
            data: OldBlockData::from_bytes(b""),
            buffer: Vec::new(),
        }
    }
}

impl ProcessBlock for I2cOutputBlock {
    type Parameters = Parameters;
    type Inputs = ByteSliceSignal;
    type Output = ByteSliceSignal;

    fn process<'b>(
        &'b mut self,
        _parameters: &Self::Parameters,
        _context: &dyn Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        self.buffer.clear();
        self.buffer.extend_from_slice(inputs);
        self.data.set_bytes(&self.buffer);
        &self.buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;

    #[test]
    fn test_i2c_output_block() {
        let mut block = I2cOutputBlock::default();
        let params = Parameters::new(64., 1.);
        let context = StubContext::default();

        let input_data: &[u8] = &[0x01, 0x02, 0x03];

        let output_signal = block.process(&params, &context, input_data);

        assert_eq!(output_signal, input_data);
        assert_eq!(block.data.to_bytes(), input_data);
    }
}
