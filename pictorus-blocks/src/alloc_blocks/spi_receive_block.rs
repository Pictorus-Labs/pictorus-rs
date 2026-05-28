use alloc::vec::Vec;
use core::time::Duration;
use pictorus_traits::{ByteSliceSignal, Context, PassBy, ProcessBlock};

use crate::stale_tracker::{duration_from_ms_f64, StaleTracker};

/// Parameters for the SPI receive block
#[doc(hidden)]
pub struct Parameters {
    /// Number of bytes to read from the SPI interface
    pub read_bytes: usize,
    /// Time after which the data is considered stale
    stale_age: Duration,
}

impl Parameters {
    pub fn new(read_bytes: f64, stale_age_ms: f64) -> Self {
        Self {
            read_bytes: read_bytes as usize,
            stale_age: duration_from_ms_f64(stale_age_ms),
        }
    }
}

/// Buffers data received from an SPI interface.
///
/// If data is not received within the time indicated in the Parameters, the data is considered stale.
/// The last valid data is kept in the buffer until new data arrives.
#[derive(Default)]
pub struct SpiReceiveBlock {
    buffer: Vec<u8>,
    stale_check: StaleTracker,
    last_valid: bool,
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
        if inputs.len() == parameters.read_bytes {
            // TODO: Error handling strategy for hardware SPI / I2C / etc. that isn't a simple buffer
            // length check.
            self.buffer.clear();
            self.buffer.extend_from_slice(inputs);
            self.stale_check.mark_updated(context.time());
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
    use core::time::Duration;

    use super::*;
    use crate::testing::StubRuntime;

    #[test]
    fn test_spi_receive_default_buffer_no_panic() {
        let block = SpiReceiveBlock::default();
        assert_eq!(block.buffer(), (b"".as_ref(), false));
    }

    #[test]
    fn test_spi_receive_block() {
        let mut block = SpiReceiveBlock::default();
        let parameters = Parameters::new(4., 100.0);
        let mut runtime = StubRuntime::default();
        let input_data = &[0x00, 0x01, 0x02, 0x03];

        // Buffer the input data
        let output = {
            let (data, valid) = block.process(&parameters, &runtime.context(), input_data);
            (data.to_vec(), valid)
        };
        assert_eq!(output.0, input_data);
        assert!(output.1);
        assert_eq!(block.buffer(), (output.0.as_slice(), output.1));

        runtime.set_time(Duration::from_secs(1));

        let (data, valid) = block.process(&parameters, &runtime.context(), &[]);
        assert_eq!(data, input_data);
        // Stale: buffered data is preserved, but `is_valid` flips to false
        assert!(!valid);
    }
}
