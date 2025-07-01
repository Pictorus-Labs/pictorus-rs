use alloc::vec::Vec;

use embassy_futures::poll_once;
use pictorus_internal::protocols::CanProtocol;
// The `fdcan` feature is mutually exclusive with the `can` feature
// due to the way embassy-stm32 implements can drivers. Boards that support
// fdcan generate different structs than boards that support standard can.
// So if we try to include both of these, the imports will fail on one or the other.
#[cfg(feature = "fdcan")]
use embassy_stm32::can::{Can, CanConfigurator, Frame, filter};
#[cfg(not(feature = "fdcan"))]
use embassy_stm32::can::{Can, Fifo, Frame, filter::Mask32};
use embedded_can::{ErrorKind, Frame as EmbeddedFrame, nb::Can as EmbeddedCan};
use pictorus_blocks::CanReceiveBlockParams;
use pictorus_blocks::CanTransmitBlockParams;
use pictorus_traits::{ByteSliceSignal, Context, InputBlock, OutputBlock, PassBy};

pub struct CanConnection<'a> {
    can: Can<'a>,
    frames: Vec<Frame>,
    stale: bool,
}

impl<'a> CanConnection<'a> {
    #[cfg(not(feature = "fdcan"))]
    pub fn new(mut can: Can<'a>, bitrate: u32) -> Self {
        can.modify_filters()
            .enable_bank(0, Fifo::Fifo0, Mask32::accept_all());

        can.modify_config().set_bitrate(bitrate);

        embassy_futures::block_on(can.enable());
        Self {
            can,
            frames: Vec::new(),
            stale: true,
        }
    }

    #[cfg(feature = "fdcan")]
    pub fn new(mut can: CanConfigurator<'a>, bitrate: u32) -> Self {
        use embassy_stm32::can::OperatingMode;

        can.properties().set_extended_filter(
            filter::ExtendedFilterSlot::_0,
            filter::ExtendedFilter::accept_all_into_fifo1(),
        );

        can.set_bitrate(bitrate);
        let can = can.start(OperatingMode::NormalOperationMode);
        Self {
            can,
            frames: Vec::new(),
            stale: true,
        }
    }
}

impl EmbeddedCan for CanConnection<'_> {
    type Frame = Frame;
    type Error = ErrorKind;

    #[cfg(not(feature = "fdcan"))]
    fn transmit(&mut self, frame: &Self::Frame) -> nb::Result<Option<Self::Frame>, Self::Error> {
        let res = self.can.try_write(frame);
        match res {
            Ok(status) => Ok(status.dequeued_frame().copied()),
            Err(_) => Err(nb::Error::WouldBlock),
        }
    }

    #[cfg(feature = "fdcan")]
    fn transmit(&mut self, frame: &Self::Frame) -> nb::Result<Option<Self::Frame>, Self::Error> {
        let res = poll_once(self.can.write(frame));
        match res {
            core::task::Poll::Ready(frame) => Ok(frame),
            core::task::Poll::Pending => Err(nb::Error::WouldBlock),
        }
    }

    fn receive(&mut self) -> nb::Result<Self::Frame, Self::Error> {
        let res = poll_once(self.can.read());
        match res {
            core::task::Poll::Ready(res) => res.map(|env| env.frame).map_err(|e| {
                let err = match e {
                    embassy_stm32::can::enums::BusError::Stuff => ErrorKind::Stuff,
                    embassy_stm32::can::enums::BusError::Form => ErrorKind::Form,
                    embassy_stm32::can::enums::BusError::Acknowledge => ErrorKind::Acknowledge,
                    embassy_stm32::can::enums::BusError::Crc => ErrorKind::Crc,
                    _ => ErrorKind::Other,
                };
                nb::Error::Other(err)
            }),
            core::task::Poll::Pending => Err(nb::Error::WouldBlock),
        }
    }
}

// This is identical to the implementation in platforms/linux/src/can_protocol.rs
// Could make this the default impl for anything that implements Can + Cache(?)
impl CanProtocol for CanConnection<'_> {
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

impl OutputBlock for CanConnection<'_> {
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

impl InputBlock for CanConnection<'_> {
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
            return &[];
        };

        frame.data()
    }
}
