use embedded_hal::i2c::{I2c, Operation};
use heapless::Vec;
use pictorus_blocks::I2cInputBlockParams;
use pictorus_traits::{ByteSliceSignal, InputBlock, OutputBlock};

pub struct I2cWrapper<const RX_BUFFER_SIZE: usize, const TX_BUFFER_SIZE: usize> {
    i2c: ra4m2_hal::i2c::I2c0,
    buffer: Vec<u8, RX_BUFFER_SIZE>,
}

impl<const RX_BUFFER_SIZE: usize, const TX_BUFFER_SIZE: usize>
    I2cWrapper<RX_BUFFER_SIZE, TX_BUFFER_SIZE>
{
    pub fn new(i2c: ra4m2_hal::i2c::I2c0) -> Self {
        I2cWrapper {
            i2c,
            buffer: Vec::new(),
        }
    }
}

impl<const RX_BUFFER_SIZE: usize, const TX_BUFFER_SIZE: usize> InputBlock
    for I2cWrapper<RX_BUFFER_SIZE, TX_BUFFER_SIZE>
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

impl<const RX_BUFFER_SIZE: usize, const TX_BUFFER_SIZE: usize> OutputBlock
    for I2cWrapper<RX_BUFFER_SIZE, TX_BUFFER_SIZE>
{
    type Inputs = ByteSliceSignal;
    type Parameters = pictorus_blocks::I2cOutputBlockParams;

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

        let result = self.i2c.transaction(
            parameters.address,
            &mut [Operation::Write(tx_buffer.as_slice())],
        );

        if result.is_err() {
            // Handle error case
        }
    }
}
