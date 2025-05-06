use std::convert::Infallible;

use embedded_can::nb::Can;
use pictorus_blocks::CanReceiveBlockParams;
use pictorus_blocks::CanTransmitBlockParams;
use pictorus_internal::protocols::Flush;
use pictorus_traits::{ByteSliceSignal, InputBlock, OutputBlock};

pub struct SimFrame {}
impl embedded_can::Frame for SimFrame {
    fn new(_id: impl Into<embedded_can::Id>, _data: &[u8]) -> Option<Self> {
        Some(Self {})
    }

    fn new_remote(_id: impl Into<embedded_can::Id>, _dlc: usize) -> Option<Self> {
        Some(Self {})
    }

    fn is_extended(&self) -> bool {
        false
    }

    fn is_remote_frame(&self) -> bool {
        false
    }

    fn id(&self) -> embedded_can::Id {
        embedded_can::Id::Standard(embedded_can::StandardId::ZERO)
    }

    fn dlc(&self) -> usize {
        0
    }

    fn data(&self) -> &[u8] {
        &[]
    }
}

pub struct SimCan {
    frame_buffer: [u8; 8],
}

impl SimCan {
    pub fn new(_iface: &str) -> Result<Self, Infallible> {
        Ok(Self {
            frame_buffer: [0; 8],
        })
    }
}

impl Can for SimCan {
    type Frame = SimFrame;
    type Error = embedded_can::ErrorKind;

    fn transmit(&mut self, _frame: &Self::Frame) -> nb::Result<Option<Self::Frame>, Self::Error> {
        Ok(None)
    }
    fn receive(&mut self) -> nb::Result<Self::Frame, Self::Error> {
        Ok(SimFrame {})
    }
}

impl Flush for SimCan {
    fn flush(&mut self) {
        // Do nothing
    }
}

impl OutputBlock for SimCan {
    type Inputs = ByteSliceSignal;

    type Parameters = CanTransmitBlockParams;

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        _inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) {
        // Do nothing
    }
}

impl InputBlock for SimCan {
    type Output = ByteSliceSignal;

    type Parameters = CanReceiveBlockParams;

    fn input(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
    ) -> pictorus_traits::PassBy<'_, Self::Output> {
        &self.frame_buffer
    }
}
