use alloc::vec::Vec;
use embedded_hal::i2c::{I2c, Operation};
use pictorus_blocks::I2cInputBlockParams;
use pictorus_traits::{ByteSliceSignal, InputBlock, OutputBlock};

pub struct I2cWrapper {
    i2c: ra4m2_hal::i2c::I2c0,
    buffer: Vec<u8>,
}

impl I2cWrapper {
    pub fn new(i2c: ra4m2_hal::i2c::I2c0) -> Self {
        I2cWrapper {
            i2c,
            buffer: Vec::new(),
        }
    }
}

impl InputBlock for I2cWrapper {
    type Output = ByteSliceSignal;
    type Parameters = I2cInputBlockParams;

    fn input(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
    ) -> pictorus_traits::PassBy<'_, Self::Output> {
        let size = parameters.read_bytes;
        self.buffer.resize(size, 0);

        // Write register, then read data
        let result = self.i2c.transaction(
            parameters.address,
            &mut [
                Operation::Write(&[parameters.command]),
                Operation::Read(&mut self.buffer[..size]),
            ],
        );

        if result.is_err() {
            // Handle error case
        }

        &self.buffer
    }
}

impl OutputBlock for I2cWrapper {
    type Inputs = ByteSliceSignal;
    type Parameters = pictorus_blocks::I2cOutputBlockParams;

    fn output(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) {
        let mut tx_buffer = Vec::new();
        tx_buffer.push(parameters.command);
        tx_buffer.extend_from_slice(inputs);

        let result = self.i2c.transaction(
            parameters.address,
            &mut [Operation::Write(tx_buffer.as_slice())],
        );

        if result.is_err() {
            // Handle error case
        }
    }
}
