use heapless::Vec;
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
#[derive(Default)]
pub struct I2cOutputBlock<const TX_BUFFER_SIZE: usize> {
    buffer: Vec<u8, TX_BUFFER_SIZE>,
}

impl<const TX_BUFFER_SIZE: usize> ProcessBlock for I2cOutputBlock<TX_BUFFER_SIZE> {
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
        if self.buffer.extend_from_slice(inputs).is_err() {
            // TODO: Error handling
        }
        &self.buffer
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        &self.buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;

    #[test]
    fn test_i2c_output_default_buffer_no_panic() {
        let block = I2cOutputBlock::<1024>::default();
        assert_eq!(block.buffer(), b"".as_ref());
    }

    #[test]
    fn test_i2c_output_block() {
        let mut block = I2cOutputBlock::<1024>::default();
        let params = Parameters::new(64., 1.);
        let context = StubContext::default();

        let input_data: &[u8] = &[0x01, 0x02, 0x03];

        let output_signal = block.process(&params, &context, input_data);

        assert_eq!(output_signal, input_data);
        assert_eq!(block.buffer(), input_data);
    }
}
