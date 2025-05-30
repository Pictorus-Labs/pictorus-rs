extern crate alloc;
use alloc::vec::Vec;
use block_data::BlockData as OldBlockData;
use pictorus_traits::{ByteSliceSignal, PassBy, ProcessBlock};

use crate::{stale_tracker::StaleTracker, IsValid};

/// Parameters for UDP Receive Block
#[doc(hidden)]
pub struct Parameters {
    pub stale_age_ms: f64,
}

impl Parameters {
    pub fn new(stale_age_ms: f64) -> Self {
        Self { stale_age_ms }
    }
}

/// Buffers data read from a UDP socket.
///
/// This block reads data from a Hardware specific UDP `InputBlock` that is added
/// by codegen. It attempts to read data each timestep and if data is available, it
/// will update its internal buffer making that data available to blocks connected to it
/// in the graph. If no data is available the buffer will remain unchanged. If no data has
/// been received for a period of time longer than the `stale_age_ms` parameter, the block
/// will return `false` for `is_valid()`.
pub struct UdpReceiveBlock {
    pub data: OldBlockData,
    pub stale_check: StaleTracker,
    buffer: Vec<u8>,
    previous_stale_check_time_ms: f64,
}

impl Default for UdpReceiveBlock {
    fn default() -> Self {
        Self {
            data: OldBlockData::from_bytes(b""),
            stale_check: StaleTracker::from_ms(0.0),
            buffer: Vec::new(),
            previous_stale_check_time_ms: 0.,
        }
    }
}

impl IsValid for UdpReceiveBlock {
    fn is_valid(&self, app_time_s: f64) -> OldBlockData {
        self.stale_check.is_valid(app_time_s)
    }
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
        if self.previous_stale_check_time_ms != parameters.stale_age_ms {
            self.stale_check = StaleTracker::from_ms(parameters.stale_age_ms);
            self.previous_stale_check_time_ms = parameters.stale_age_ms;
        }

        // Make sure the data is the correct size, if so, update the stale check, otherwise
        // something has gone wrong.
        if !input.is_empty() {
            self.stale_check.mark_updated(context.time().as_secs_f64());
            self.buffer = input.to_vec();
            self.data.set_bytes(&self.buffer);
        }

        (
            &self.buffer,
            self.stale_check.is_valid_bool(context.time().as_secs_f64()),
        )
    }
}
