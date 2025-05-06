extern crate alloc;
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use block_data::BlockData as OldBlockData;
use pictorus_traits::{ByteSliceSignal, PassBy, ProcessBlock};

/// Parameters for UDP Transmit Block
pub struct Parameters {
    /// Destination address for the UDP socket
    /// e.g. "192.168.0.1:12345"
    destination: String,
}

impl Parameters {
    pub fn new(destination: &str) -> Self {
        Self {
            destination: destination.to_string(),
        }
    }

    /// Get the destination address for the UDP socket
    pub fn destination(&self) -> &str {
        &self.destination
    }
}

/// UDP Transmit Block sends serial data to a UDP socket.
/// This block sends data to a Hardware specific UDP `OutputBlock` that is added
/// by codegen
pub struct UdpTransmitBlock {
    pub data: OldBlockData,
    buffer: Vec<u8>,
}

impl Default for UdpTransmitBlock {
    fn default() -> Self {
        Self {
            data: OldBlockData::from_bytes(b""),
            buffer: Vec::new(),
        }
    }
}

impl ProcessBlock for UdpTransmitBlock {
    type Parameters = Parameters;
    type Inputs = ByteSliceSignal;
    type Output = ByteSliceSignal;

    fn process<'b>(
        &'b mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        self.buffer.clear();
        self.buffer.extend_from_slice(inputs);
        self.data.set_bytes(&self.buffer);
        &self.buffer
    }
}
