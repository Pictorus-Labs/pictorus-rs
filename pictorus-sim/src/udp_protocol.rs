use std::{convert::Infallible, io::Error};

use pictorus_blocks::{UdpReceiveBlockParams, UdpTransmitBlockParams};
use pictorus_traits::{ByteSliceSignal, InputBlock, OutputBlock};

use pictorus_internal::protocols::UdpProtocol;

pub struct UdpConnection {}

impl UdpConnection {
    pub fn new(_addr: &str, _transmit_enabled: bool) -> Result<Self, Infallible> {
        Ok(UdpConnection {})
    }
}

impl UdpProtocol for UdpConnection {
    fn read(&mut self) -> Result<&[u8], Error> {
        Ok(&[])
    }

    fn write(&mut self, buf: &[u8], _to_addr: &str) -> Result<usize, Error> {
        Ok(buf.len())
    }

    fn flush(&mut self) {}
}

pub fn create_udp_socket(_address: &str, _transmit_enabled: bool) -> UdpConnection {
    UdpConnection {}
}

impl InputBlock for UdpConnection {
    type Output = ByteSliceSignal;
    type Parameters = UdpReceiveBlockParams;

    fn input(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
    ) -> pictorus_traits::PassBy<'_, Self::Output> {
        &[]
    }
}

impl OutputBlock for UdpConnection {
    type Parameters = UdpTransmitBlockParams;
    type Inputs = ByteSliceSignal;

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        _inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) {
    }
}
