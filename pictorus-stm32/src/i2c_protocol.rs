use alloc::vec::Vec;
use embassy_stm32::i2c::I2c;
use embassy_stm32::mode::Blocking;
use embedded_hal::i2c::I2c as I2cTrait;
use pictorus_blocks::{I2cInputBlockParams, I2cOutputBlockParams};
use pictorus_traits::{ByteSliceSignal, InputBlock, OutputBlock};

pub struct I2cWrapper<'a> {
    i2c: I2c<'a, Blocking>,
    buffer: Vec<u8>,
}

impl<'a> I2cWrapper<'a> {
    pub fn new(i2c: I2c<'a, Blocking>) -> Self {
        Self {
            i2c,
            buffer: Vec::new(),
        }
    }
}

impl InputBlock for I2cWrapper<'_> {
    type Output = ByteSliceSignal;
    type Parameters = I2cInputBlockParams;

    fn input(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
    ) -> pictorus_traits::PassBy<'_, Self::Output> {
        let size = parameters.read_bytes;
        self.buffer.resize(size, 0);
        let result = self.i2c.write_read(
            parameters.address,
            &[parameters.command],
            &mut self.buffer[..size],
        );

        if result.is_err() {
            // TODO: Error handling
            // Keep the results, good or bad, in memory
        }

        &self.buffer
    }
}

impl OutputBlock for I2cWrapper<'_> {
    type Inputs = ByteSliceSignal;
    type Parameters = I2cOutputBlockParams;

    fn output(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) {
        let mut tx_buffer = Vec::new();
        tx_buffer.push(parameters.command);
        tx_buffer.extend_from_slice(inputs);
        self.i2c.write(parameters.address, &tx_buffer).ok();
    }
}
