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

pub struct I2cWrapper {
    pub i2c: I2cdev,
    buffer: Vec<u8>,
}

impl I2cWrapper {
    pub fn new() -> Self {
        let i2c = create_i2c_protocol().expect("I2C device not found");

        Self {
            i2c,
            buffer: Vec::new(),
        }
    }
}

impl Default for I2cWrapper {
    fn default() -> Self {
        Self::new()
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
        self.buffer.resize(parameters.read_bytes, 0);
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

impl OutputBlock for I2cWrapper {
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
