extern crate alloc;
use alloc::vec::Vec;
use pictorus_block_data::BlockData as OldBlockData;
use pictorus_traits::{ByteSliceSignal, Context, PassBy, ProcessBlock};

use crate::{stale_tracker::StaleTracker, IsValid};

/// Parameters for the SPI receive block
#[doc(hidden)]
pub struct Parameters {
    /// Number of bytes to read from the SPI interface
    pub read_bytes: usize,
    /// Time in milliseconds after which the data is considered stale
    stale_age_ms: f64,
}

impl Parameters {
    pub fn new(read_bytes: f64, stale_age_ms: f64) -> Self {
        Self {
            read_bytes: read_bytes as usize,
            stale_age_ms,
        }
    }
}

/// Buffers data received from an SPI interface.
///
/// If data is not received within the time indicated in the Parameters, the data is considered stale.
/// The last valid data is kept in the buffer until new data arrives.
pub struct SpiReceiveBlock {
    pub data: OldBlockData,
    buffer: Vec<u8>,
    pub stale_check: StaleTracker,
    previous_stale_check_time_ms: f64,
}

impl Default for SpiReceiveBlock {
    fn default() -> Self {
        Self {
            data: OldBlockData::from_bytes(&[]),
            buffer: Vec::new(),
            stale_check: StaleTracker::from_ms(0.),
            previous_stale_check_time_ms: 0.0,
        }
    }
}

impl ProcessBlock for SpiReceiveBlock {
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

        if inputs.len() == parameters.read_bytes {
            // TODO: Error handling strategy for hardware SPI / I2C / etc. that isn't a simple buffer
            // length check.
            self.buffer.clear();
            self.buffer.extend_from_slice(inputs);
            self.data.set_bytes(&self.buffer);
            self.stale_check.mark_updated(context.time().as_secs_f64());
        }

        (
            &self.buffer,
            self.stale_check.is_valid_bool(context.time().as_secs_f64()),
        )
    }
}

impl IsValid for SpiReceiveBlock {
    fn is_valid(&self, app_time_s: f64) -> OldBlockData {
        self.stale_check.is_valid(app_time_s)
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use super::*;
    use crate::testing::StubRuntime;
    use pictorus_traits::Context;

    #[test]
    fn test_spi_receive_block() {
        let mut block = SpiReceiveBlock::default();
        let parameters = Parameters::new(4., 100.0);
        let mut runtime = StubRuntime::default();
        let input_data = &[0x00, 0x01, 0x02, 0x03];

        // Buffer the input data
        let output = block.process(&parameters, &runtime.context(), input_data);
        assert_eq!(output.0, input_data);
        assert_eq!(block.data.to_bytes(), input_data);
        let is_valid = block
            .stale_check
            .is_valid(runtime.context().time().as_secs_f64());
        assert_eq!(is_valid, OldBlockData::from_scalar(1.0));

        runtime.set_time(Duration::from_secs(1));

        let output = block.process(&parameters, &runtime.context(), &[]);
        assert_eq!(output.0, input_data);
        assert_eq!(block.data.to_bytes(), input_data);
        let is_valid = block
            .stale_check
            .is_valid(runtime.context().time().as_secs_f64());
        // However block should be invalid
        assert_eq!(is_valid, OldBlockData::from_scalar(0.0));
    }
}
