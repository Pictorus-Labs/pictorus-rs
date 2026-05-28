use core::time::Duration;

use alloc::vec::Vec;
use pictorus_traits::{ByteSliceSignal, Context, PassBy, ProcessBlock};

use crate::stale_tracker::StaleTracker;

/// Parameters for I2C Input Block
#[doc(hidden)]
pub struct Parameters {
    /// 8-bit address to read from
    pub address: u8,
    /// 8-bit command to send, typically a register address
    pub command: u8,
    /// Number of bytes to read from the I2C device
    pub read_bytes: usize,
    /// Stale age
    stale_age: Duration,
}

impl Parameters {
    pub fn new(address: f64, command: f64, read_bytes: f64, stale_age_ms: f64) -> Self {
        let addr_u8 = address as u8;
        let command_u8 = command as u8;
        let read_bytes_u8 = read_bytes as usize;

        Self {
            address: addr_u8,
            command: command_u8,
            read_bytes: read_bytes_u8,
            stale_age: Duration::from_secs_f64(stale_age_ms / 1000.0),
        }
    }
}

/// I2C Input Block buffers data read from an I2C peripheral.
#[derive(Default)]
pub struct I2cInputBlock {
    stale_check: StaleTracker,
    buffer: Vec<u8>,
    last_valid: bool,
}

impl ProcessBlock for I2cInputBlock {
    type Parameters = Parameters;
    type Inputs = ByteSliceSignal;
    type Output = (ByteSliceSignal, bool);

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        context: &dyn Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        // Make sure the data is the correct size, if so, update the stale check, otherwise
        // something has gone wrong.
        if inputs.len() == parameters.read_bytes {
            self.buffer.clear();
            self.stale_check.mark_updated(context.time());
            self.buffer.extend_from_slice(inputs);
        }

        self.last_valid = self
            .stale_check
            .is_valid(context.time(), parameters.stale_age);
        (&self.buffer, self.last_valid)
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        (&self.buffer, self.last_valid)
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::StubRuntime;
    use core::time::Duration;

    use super::*;

    #[test]
    fn test_i2c_input_default_buffer_no_panic() {
        let block = I2cInputBlock::default();
        assert_eq!(block.buffer(), (b"".as_ref(), false));
    }

    #[test]
    fn test_i2c_input_block() {
        let parameters = Parameters::new(0., 0., 10., 100.0);
        let mut runtime = StubRuntime::default();
        let mut block = I2cInputBlock::default();

        let input_data: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        let output = {
            let (data, valid) = block.process(&parameters, &runtime.context(), input_data);
            (data.to_vec(), valid)
        };
        assert_eq!(output.0, input_data);
        assert!(output.1);
        assert_eq!(block.buffer(), (output.0.as_slice(), output.1));

        runtime.set_time(Duration::from_secs(1));

        // When the I2cWrapper has an error, the buffer is clear and the parameters.read_bytes is not
        // equal to the length of the empty buffer, however the previous value is buffered
        let (data, valid) = block.process(&parameters, &runtime.context(), &[]);
        assert_eq!(data, input_data);
        assert!(!valid);
    }
}
