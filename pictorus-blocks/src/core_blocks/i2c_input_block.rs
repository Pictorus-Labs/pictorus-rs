extern crate alloc;
use alloc::vec::Vec;
use block_data::BlockData as OldBlockData;
use pictorus_traits::{ByteSliceSignal, Context, PassBy, ProcessBlock};

use crate::{stale_tracker::StaleTracker, IsValid};

/// Parameters for I2C Input Block
#[doc(hidden)]
pub struct Parameters {
    /// 8-bit address to read from
    pub address: u8,
    /// 8-bit command to send, typically a register address
    pub command: u8,
    /// Number of bytes to read from the I2C device
    pub read_bytes: usize,
    /// Stale age in milliseconds
    stale_age_ms: f64,
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
            stale_age_ms,
        }
    }
}

/// I2C Input Block buffers data read from an I2C peripheral.
pub struct I2cInputBlock {
    pub data: OldBlockData,
    pub stale_check: StaleTracker,
    buffer: Vec<u8>,
    previous_stale_check_time_ms: f64,
}

impl Default for I2cInputBlock {
    fn default() -> Self {
        Self {
            data: OldBlockData::from_bytes(b""),
            stale_check: StaleTracker::from_ms(0.0),
            buffer: Vec::new(),
            previous_stale_check_time_ms: 0.,
        }
    }
}

impl IsValid for I2cInputBlock {
    fn is_valid(&self, app_time_s: f64) -> OldBlockData {
        self.stale_check.is_valid(app_time_s)
    }
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
        if self.previous_stale_check_time_ms != parameters.stale_age_ms {
            self.stale_check = StaleTracker::from_ms(parameters.stale_age_ms);
            self.previous_stale_check_time_ms = parameters.stale_age_ms;
        }

        // Make sure the data is the correct size, if so, update the stale check, otherwise
        // something has gone wrong.
        if inputs.len() == parameters.read_bytes {
            self.buffer.clear();
            self.stale_check.mark_updated(context.time().as_secs_f64());
            self.buffer.extend_from_slice(inputs);
            self.data.set_bytes(&self.buffer);
        }

        (
            &self.buffer,
            self.stale_check.is_valid_bool(context.time().as_secs_f64()),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::StubRuntime;
    use core::time::Duration;

    use super::*;

    #[test]
    fn test_i2c_input_block() {
        let parameters = Parameters::new(0., 0., 10., 100.0);
        let mut runtime = StubRuntime::default();
        let mut block = I2cInputBlock::default();

        let input_data: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        let output = block.process(&parameters, &runtime.context(), input_data);
        assert_eq!(output.0, input_data);
        assert_eq!(block.data.to_bytes(), input_data);
        let valid = block
            .is_valid(runtime.context().time().as_secs_f64())
            .clone();
        assert_eq!(valid.scalar(), 1.0);

        runtime.set_time(Duration::from_secs(1));

        // When the I2cWrapper has an error, the buffer is clear and the parameters.read_bytes is not
        // equal to the length of the empty buffer, however the previous value is buffered
        let output = block.process(&parameters, &runtime.context(), &[]);
        assert_eq!(output.0, input_data);
        let valid = block.is_valid(runtime.context().time().as_secs_f64());
        assert_eq!(valid.scalar(), 0.0);
    }
}
