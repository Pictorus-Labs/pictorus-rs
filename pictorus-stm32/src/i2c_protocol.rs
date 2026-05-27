use embassy_stm32::i2c::I2c;
use embassy_stm32::mode::Blocking;
use embedded_hal::i2c::I2c as I2cTrait;
use heapless::Vec;
use pictorus_blocks::{I2cInputBlockParams, I2cOutputBlockParams};
use pictorus_traits::{ByteSliceSignal, InputBlock, OutputBlock};

pub struct I2cWrapper<'a, const RX_BUFFER_SIZE: usize, const TX_BUFFER_SIZE: usize> {
    i2c: I2c<'a, Blocking>,
    buffer: Vec<u8, RX_BUFFER_SIZE>,
}

impl<'a, const RX_BUFFER_SIZE: usize, const TX_BUFFER_SIZE: usize>
    I2cWrapper<'a, RX_BUFFER_SIZE, TX_BUFFER_SIZE>
{
    pub fn new(i2c: I2c<'a, Blocking>) -> Self {
        Self {
            i2c,
            buffer: Vec::new(),
        }
    }
}

impl<const RX_BUFFER_SIZE: usize, const TX_BUFFER_SIZE: usize> InputBlock
    for I2cWrapper<'_, RX_BUFFER_SIZE, TX_BUFFER_SIZE>
{
    type Output = ByteSliceSignal;
    type Parameters = I2cInputBlockParams;

    fn input(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
    ) -> pictorus_traits::PassBy<'_, Self::Output> {
        let size = parameters.read_bytes;
        if self.buffer.resize(size, 0).is_err() {
            // TODO: Error handling
        }

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

impl<const RX_BUFFER_SIZE: usize, const TX_BUFFER_SIZE: usize> OutputBlock
    for I2cWrapper<'_, RX_BUFFER_SIZE, TX_BUFFER_SIZE>
{
    type Inputs = ByteSliceSignal;
    type Parameters = I2cOutputBlockParams;

    fn output(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) {
        let mut tx_buffer: Vec<u8, TX_BUFFER_SIZE> = Vec::new();
        if tx_buffer.push(parameters.command).is_err() {
            // TODO: Error handling
        }
        if tx_buffer.extend_from_slice(inputs).is_err() {
            // TODO: Error handling
        }
        self.i2c.write(parameters.address, &tx_buffer).ok();
    }
}
