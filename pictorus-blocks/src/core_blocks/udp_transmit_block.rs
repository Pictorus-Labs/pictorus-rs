extern crate alloc;
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use pictorus_traits::{ByteSliceSignal, PassBy, ProcessBlock};

/// Parameters for UDP Transmit Block
#[doc(hidden)]
pub struct Parameters {
    /// Destination address for the UDP socket
    /// e.g. "192.168.0.1:12345"
    destination: String,
}

impl Parameters {
    pub fn new(destination: &[u8]) -> Self {
        Self {
            destination: String::from_utf8_lossy(destination).to_string(),
        }
    }

    /// Get the destination address for the UDP socket
    pub fn destination(&self) -> &str {
        &self.destination
    }
}

/// Buffers data to be sent to a UDP port.
///
/// This block sends data to a Hardware specific UDP `OutputBlock` that is added
/// by codegen
pub struct UdpTransmitBlock {
    buffer: Vec<u8>,
}

impl Default for UdpTransmitBlock {
    fn default() -> Self {
        Self { buffer: Vec::new() }
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
        &self.buffer
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        &self.buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_udp_transmit_default_buffer_no_panic() {
        let block = UdpTransmitBlock::default();
        assert_eq!(block.buffer(), b"".as_ref());
    }
}
