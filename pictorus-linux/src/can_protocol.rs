use embedded_can::{Frame as EmbeddedFrame, nb::Can};
use log::debug;
use pictorus_blocks::CanReceiveBlockParams;
use pictorus_blocks::CanTransmitBlockParams;
use pictorus_traits::{ByteSliceSignal, Context, InputBlock, OutputBlock, PassBy};
use socketcan::{CanFrame, CanSocket, Socket};

use pictorus_internal::protocols::CanProtocol;
use pictorus_internal::utils::PictorusError;

const ERR_TYPE: &str = "CanProtocol";

pub struct CanConnection {
    socket: CanSocket,
    frames: Vec<CanFrame>,
    stale: bool,
}

impl CanConnection {
    pub fn new(iface: &str) -> Result<Self, PictorusError> {
        let socket = CanSocket::open(iface).map_err(|err| {
            PictorusError::new(
                ERR_TYPE.into(),
                format!("Failed to open CAN socket on interface: {iface} ({err})",),
            )
        })?;

        socket.set_nonblocking(true).map_err(|err| {
            PictorusError::new(
                ERR_TYPE.into(),
                format!("Failed to set CAN socket to non-blocking mode: {iface} ({err})",),
            )
        })?;

        Ok(Self {
            socket,
            frames: vec![],
            stale: true,
        })
    }
}

impl Can for CanConnection {
    type Frame = CanFrame;
    type Error = socketcan::Error;

    fn transmit(&mut self, frame: &Self::Frame) -> nb::Result<Option<Self::Frame>, Self::Error> {
        self.socket.transmit(frame)
    }

    fn receive(&mut self) -> nb::Result<Self::Frame, Self::Error> {
        self.socket.receive()
    }
}

impl CanProtocol for CanConnection {
    fn read_frames(&mut self) -> &[impl EmbeddedFrame] {
        if !self.stale {
            return &self.frames;
        }

        while let Ok(frame) = self.receive() {
            self.frames.push(frame);
        }

        self.stale = false;
        &self.frames
    }

    fn flush(&mut self) {
        self.stale = true;
        self.frames.clear();
    }
}

impl OutputBlock for CanConnection {
    type Inputs = ByteSliceSignal;

    type Parameters = CanTransmitBlockParams;

    fn output(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) {
        let Some(frame) = EmbeddedFrame::new(parameters.frame_id, inputs) else {
            log::warn!("Failed to create frame");
            return;
        };

        let res = self.transmit(&frame);
        if let Err(e) = res {
            log::warn!("Failed to transmit frame: {e:?}");
        }
    }
}

impl InputBlock for CanConnection {
    type Output = ByteSliceSignal;

    type Parameters = CanReceiveBlockParams;

    fn input(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
    ) -> pictorus_traits::PassBy<'_, Self::Output> {
        let frame = self
            .read_frames()
            .iter()
            .rfind(|frame| frame.id() == parameters.frame_id);

        let Some(frame) = frame else {
            debug!("No Frames to process");
            return &[];
        };

        frame.data()
    }
}
