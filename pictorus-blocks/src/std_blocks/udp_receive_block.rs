use alloc::vec::Vec;
use core::time::Duration;
use pictorus_traits::{ByteSliceSignal, PassBy, ProcessBlock};

use crate::stale_tracker::{duration_from_ms_f64, StaleTracker};

/// Parameters for UDP Receive Block
#[doc(hidden)]
pub struct Parameters {
    pub stale_age: Duration,
}

impl Parameters {
    pub fn new(stale_age_ms: f64) -> Self {
        Self {
            stale_age: duration_from_ms_f64(stale_age_ms),
        }
    }
}

/// Buffers data read from a UDP socket.
///
/// This block reads data from a Hardware specific UDP `InputBlock` that is added
/// by codegen. It attempts to read data each timestep and if data is available, it
/// will update its internal buffer making that data available to blocks connected to it
/// in the graph. If no data is available the buffer will remain unchanged. If no data has
/// been received for a period longer than the `stale_age` parameter, the block's trailing
/// output bool flips to `false`.
#[derive(Default)]
pub struct UdpReceiveBlock {
    stale_check: StaleTracker,
    buffer: Vec<u8>,
    last_valid: bool,
}

impl ProcessBlock for UdpReceiveBlock {
    type Parameters = Parameters;
    type Inputs = ByteSliceSignal;
    type Output = (ByteSliceSignal, bool);

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        context: &dyn pictorus_traits::Context,
        input: PassBy<'_, Self::Inputs>,
    ) -> pictorus_traits::PassBy<'b, Self::Output> {
        // Make sure the data is the correct size, if so, update the stale check, otherwise
        // something has gone wrong.
        if !input.is_empty() {
            self.stale_check.mark_updated(context.time());
            self.buffer = input.to_vec();
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
    use super::*;

    #[test]
    fn test_udp_receive_default_buffer_no_panic() {
        let block = UdpReceiveBlock::default();
        assert_eq!(block.buffer(), (b"".as_ref(), false));
    }
}
