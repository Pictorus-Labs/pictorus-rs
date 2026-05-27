pub use embedded_hal_02::blocking::i2c::{Write, WriteRead};
pub use linux_embedded_hal::I2cdev;
use linux_embedded_hal::i2cdev::linux::LinuxI2CError;
use pictorus_blocks::{I2cInputBlockParams, I2cOutputBlockParams};
use pictorus_traits::{ByteSliceSignal, InputBlock, OutputBlock};

use pictorus_internal::protocols::I2c;
use pictorus_internal::utils::PictorusError;

const ERR_TYPE: &str = "I2cProtocol";
// TODO: This should be configurable by block param
const I2C_PATH: &str = "/dev/i2c-1";

pub fn create_i2c_protocol() -> Result<I2cdev, PictorusError> {
    let i2c = I2cdev::new(I2C_PATH).map_err(|err| {
        let msg = match err {
            LinuxI2CError::Errno(e) => {
                format!("Unknown error! Failed to bind to I2C device: {I2C_PATH} ({e})",)
            }
            LinuxI2CError::Io(e) => match e.kind() {
                std::io::ErrorKind::NotFound => format!(
                    "Failed to bind to I2C device: {I2C_PATH} - not found. Is the I2C bus enabled?",
                ),
                _ => format!("Unknown error! Failed to bind to I2C device: {I2C_PATH} ({e})",),
            },
        };
        PictorusError::new(ERR_TYPE.into(), msg)
    })?;

    Ok(i2c)
}

pub struct I2cWrapper<const RX_BUFFER_SIZE: usize, const TX_BUFFER_SIZE: usize> {
    pub i2c: I2cdev,
    buffer: heapless::Vec<u8, RX_BUFFER_SIZE>,
}

impl<const RX_BUFFER_SIZE: usize, const TX_BUFFER_SIZE: usize>
    I2cWrapper<RX_BUFFER_SIZE, TX_BUFFER_SIZE>
{
    pub fn new() -> Self {
        let i2c = create_i2c_protocol().expect("I2C device not found");

        Self {
            i2c,
            buffer: heapless::Vec::<u8, RX_BUFFER_SIZE>::new(),
        }
    }
}

impl<const RX_BUFFER_SIZE: usize, const TX_BUFFER_SIZE: usize> Default
    for I2cWrapper<RX_BUFFER_SIZE, TX_BUFFER_SIZE>
{
    fn default() -> Self {
        Self::new()
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
        let result = self.i2c.write_read(
            parameters.address,
            &[parameters.command],
            &mut self.buffer[..size],
        );

        if result.is_err() {
            // TODO: Error handling
            // Keep results, good or bad, in memory
        }

        &self.buffer
    }
}

impl<const RX_BUFFER_SIZE: usize, const TX_BUFFER_SIZE: usize> OutputBlock
    for I2cWrapper<RX_BUFFER_SIZE, TX_BUFFER_SIZE>
{
    type Inputs = ByteSliceSignal;
    type Parameters = I2cOutputBlockParams;

    fn output(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) {
        let mut tx_buffer: heapless::Vec<u8, TX_BUFFER_SIZE> = heapless::Vec::new();
        if tx_buffer.push(parameters.command).is_err() {
            // TODO: Error handling
        }
        if tx_buffer.extend_from_slice(inputs).is_err() {
            // TODO: Error handling
        }
        self.i2c.write(parameters.address, &tx_buffer).ok();
    }
}
