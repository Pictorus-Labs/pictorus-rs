use core::ffi::c_void;

use alloc::vec::Vec;
use embedded_can::Id;
use pictorus_blocks::{CanReceiveBlockParams, CanTransmitBlockParams};
use pictorus_internal::protocols::Flush;
use pictorus_traits::{ByteSliceSignal, InputBlock, ModelClock, OutputBlock, PassBy};

use super::{Peripheral, pictorus_peripheral_status_t};
use crate::status::{
    PICTORUS_EMPTY, PICTORUS_ERR_INVALID_ARG, PICTORUS_ERR_PERIPHERAL, PICTORUS_OK,
    pictorus_status_t,
};

const MAX_STANDARD_ID: u32 = 0x7FF;
const MAX_EXTENDED_ID: u32 = 0x1FFF_FFFF;
const MAX_DLC: u8 = 8;

/// The most frames taken from C in one tick. Any beyond that stay with C for the next tick.
pub const RX_FRAMES_PER_TICK: usize = 64;

/// Mirror of `pictorus_can_frame_t` in `pictorus_ffi.h`. A classic CAN data frame.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct pictorus_can_frame_t {
    /// 11 bit identifier, or 29 bit if `extended`.
    pub id: u32,
    pub extended: bool,
    /// Number of valid bytes in `data`, at most 8.
    pub dlc: u8,
    pub data: [u8; 8],
}

impl pictorus_can_frame_t {
    fn is_valid(&self) -> bool {
        let max_id = if self.extended {
            MAX_EXTENDED_ID
        } else {
            MAX_STANDARD_ID
        };
        self.dlc <= MAX_DLC && self.id <= max_id
    }

    fn data(&self) -> &[u8] {
        &self.data[..usize::from(self.dlc)]
    }
}

/// Mirror of `pictorus_can_t` in `pictorus_ffi.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct pictorus_can_t {
    pub ctx: *mut c_void,
    pub receive: Option<
        unsafe extern "C" fn(
            ctx: *mut c_void,
            frame: *mut pictorus_can_frame_t,
        ) -> pictorus_status_t,
    >,
    pub transmit: Option<
        unsafe extern "C" fn(
            ctx: *mut c_void,
            frame: *const pictorus_can_frame_t,
        ) -> pictorus_status_t,
    >,
}

fn split_id(id: Id) -> (u32, bool) {
    match id {
        Id::Standard(id) => (u32::from(id.as_raw()), false),
        Id::Extended(id) => (id.as_raw(), true),
    }
}

/// A CAN bus backed by C.
///
/// Frames are drained from C into a per-tick cache the first time a block reads the bus, so every
/// block sees the same frames. A receive block takes the latest frame with its identifier. At
/// most [`RX_FRAMES_PER_TICK`] are taken per tick, and any beyond that stay with C for the next
/// one. Decoding signals from the frame is up to the blocks.
pub struct CanBus {
    handle: pictorus_can_t,
    frames: Vec<pictorus_can_frame_t>,
    polled: bool,
    status: pictorus_peripheral_status_t,
}

impl CanBus {
    /// # Safety
    ///
    /// `handle.ctx` must remain valid for as long as the adapter lives, and the functions in
    /// `handle`, if set, must be sound to call with it.
    pub unsafe fn new(handle: pictorus_can_t) -> Self {
        Self {
            handle,
            frames: Vec::new(),
            polled: false,
            status: Default::default(),
        }
    }

    fn drain_receive_queue(&mut self) {
        self.polled = true;
        // Counted by calls, not frames kept, so that a C side that keeps reporting invalid
        // frames cannot hold the tick up.
        for _ in 0..RX_FRAMES_PER_TICK {
            let mut frame = pictorus_can_frame_t::default();
            let status = match self.handle.receive {
                // SAFETY: guaranteed by the contract of `new`.
                Some(receive) => unsafe { receive(self.handle.ctx, &mut frame) },
                None => PICTORUS_ERR_PERIPHERAL,
            };

            if status == PICTORUS_OK {
                if frame.is_valid() {
                    self.frames.push(frame);
                } else {
                    self.status.record(PICTORUS_ERR_INVALID_ARG);
                }
            } else {
                if status != PICTORUS_EMPTY {
                    self.status.record(status);
                }
                break;
            }
        }
    }
}

impl Peripheral for CanBus {
    fn status(&self) -> pictorus_peripheral_status_t {
        self.status
    }
}

impl InputBlock for CanBus {
    type Output = ByteSliceSignal;
    type Parameters = CanReceiveBlockParams;

    fn input(
        &mut self,
        parameters: &Self::Parameters,
        _model_clock: &dyn ModelClock,
    ) -> PassBy<'_, Self::Output> {
        if !self.polled {
            self.drain_receive_queue();
        }

        let (id, extended) = split_id(parameters.frame_id);
        self.frames
            .iter()
            .rfind(|frame| frame.id == id && frame.extended == extended)
            .map_or(&[], pictorus_can_frame_t::data)
    }
}

impl OutputBlock for CanBus {
    type Inputs = ByteSliceSignal;
    type Parameters = CanTransmitBlockParams;

    fn output(
        &mut self,
        parameters: &Self::Parameters,
        _model_clock: &dyn ModelClock,
        inputs: PassBy<'_, Self::Inputs>,
    ) {
        let (id, extended) = split_id(parameters.frame_id);
        let Ok(dlc) = u8::try_from(inputs.len()) else {
            log::warn!("Failed to create frame");
            return;
        };
        if dlc > MAX_DLC {
            log::warn!("Failed to create frame");
            return;
        }

        let mut frame = pictorus_can_frame_t {
            id,
            extended,
            dlc,
            data: [0; 8],
        };
        frame.data[..inputs.len()].copy_from_slice(inputs);

        let status = match self.handle.transmit {
            // SAFETY: guaranteed by the contract of `new`.
            Some(transmit) => unsafe { transmit(self.handle.ctx, &frame) },
            None => PICTORUS_ERR_PERIPHERAL,
        };
        self.status.record(status);
    }
}

impl Flush for CanBus {
    fn flush(&mut self) {
        self.polled = false;
        self.frames.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peripherals::test_util::Clock;
    use core::cell::{Cell, RefCell};
    use embedded_can::{ExtendedId, StandardId};
    use std::collections::VecDeque;

    // The tests change the fake while the adapter holds a pointer to it, hence the cells.
    struct Fake {
        queue: RefCell<VecDeque<pictorus_can_frame_t>>,
        receive_result: Cell<pictorus_status_t>,
        receives: Cell<usize>,
        sent: RefCell<Vec<pictorus_can_frame_t>>,
        transmit_result: Cell<pictorus_status_t>,
    }

    impl Fake {
        fn new() -> Self {
            Self {
                queue: RefCell::new(VecDeque::new()),
                receive_result: Cell::new(PICTORUS_OK),
                receives: Cell::new(0),
                sent: RefCell::new(Vec::new()),
                transmit_result: Cell::new(PICTORUS_OK),
            }
        }

        fn push(&self, id: u32, extended: bool, data: &[u8]) {
            let mut frame = pictorus_can_frame_t {
                id,
                extended,
                dlc: data.len() as u8,
                data: [0; 8],
            };
            frame.data[..data.len()].copy_from_slice(data);
            self.queue.borrow_mut().push_back(frame);
        }
    }

    unsafe extern "C" fn receive(
        ctx: *mut c_void,
        frame: *mut pictorus_can_frame_t,
    ) -> pictorus_status_t {
        let fake = unsafe { &*(ctx as *const Fake) };
        fake.receives.set(fake.receives.get() + 1);
        if fake.receive_result.get() != PICTORUS_OK {
            return fake.receive_result.get();
        }
        match fake.queue.borrow_mut().pop_front() {
            Some(next) => {
                unsafe { *frame = next };
                PICTORUS_OK
            }
            None => PICTORUS_EMPTY,
        }
    }

    unsafe extern "C" fn transmit(
        ctx: *mut c_void,
        frame: *const pictorus_can_frame_t,
    ) -> pictorus_status_t {
        let fake = unsafe { &*(ctx as *const Fake) };
        fake.sent.borrow_mut().push(unsafe { *frame });
        fake.transmit_result.get()
    }

    fn bus(fake: &Fake) -> CanBus {
        unsafe {
            CanBus::new(pictorus_can_t {
                ctx: fake as *const Fake as *mut c_void,
                receive: Some(receive),
                transmit: Some(transmit),
            })
        }
    }

    fn std_id(id: u16) -> Id {
        Id::Standard(StandardId::new(id).unwrap())
    }

    fn ext_id(id: u32) -> Id {
        Id::Extended(ExtendedId::new(id).unwrap())
    }

    fn rx_params(id: Id) -> CanReceiveBlockParams {
        CanReceiveBlockParams::new(id, 8, 0.0)
    }

    #[test]
    fn takes_the_latest_frame_with_the_matching_id() {
        let fake = Fake::new();
        fake.push(0x100, false, &[1]);
        fake.push(0x200, false, &[2]);
        fake.push(0x100, false, &[3, 4]);
        let mut bus = bus(&fake);

        assert_eq!(bus.input(&rx_params(std_id(0x100)), &Clock), [3, 4]);
        assert_eq!(bus.input(&rx_params(std_id(0x200)), &Clock), [2]);
        assert!(bus.input(&rx_params(std_id(0x300)), &Clock).is_empty());
    }

    #[test]
    fn standard_and_extended_ids_are_distinct() {
        let fake = Fake::new();
        fake.push(0x100, true, &[7]);
        let mut bus = bus(&fake);

        assert!(bus.input(&rx_params(std_id(0x100)), &Clock).is_empty());
        assert_eq!(bus.input(&rx_params(ext_id(0x100)), &Clock), [7]);
    }

    #[test]
    fn drains_once_per_tick_until_empty() {
        let fake = Fake::new();
        fake.push(1, false, &[1]);
        fake.push(1, false, &[2]);
        let mut bus = bus(&fake);
        let params = rx_params(std_id(1));

        assert_eq!(bus.input(&params, &Clock), [2]);
        // Two frames plus the call that reported the queue empty.
        assert_eq!(fake.receives.get(), 3);
        assert_eq!(bus.input(&params, &Clock), [2]);
        assert_eq!(fake.receives.get(), 3);

        bus.flush();
        fake.push(1, false, &[9]);
        assert_eq!(bus.input(&params, &Clock), [9]);
    }

    #[test]
    fn leaves_excess_frames_with_c_for_the_next_tick() {
        let fake = Fake::new();
        for i in 0..RX_FRAMES_PER_TICK as u8 + 3 {
            fake.push(1, false, &[i]);
        }
        let mut bus = bus(&fake);
        let params = rx_params(std_id(1));

        assert_eq!(bus.input(&params, &Clock), [RX_FRAMES_PER_TICK as u8 - 1]);
        assert_eq!(fake.queue.borrow().len(), 3);
        bus.flush();
        assert_eq!(bus.input(&params, &Clock), [RX_FRAMES_PER_TICK as u8 + 2]);
    }

    #[test]
    fn a_flood_of_invalid_frames_cannot_hold_up_the_tick() {
        let fake = Fake::new();
        for _ in 0..RX_FRAMES_PER_TICK * 2 {
            fake.queue.borrow_mut().push_back(pictorus_can_frame_t {
                id: 0x800,
                extended: false,
                dlc: 0,
                data: [0; 8],
            });
        }
        let mut bus = bus(&fake);

        assert!(bus.input(&rx_params(std_id(1)), &Clock).is_empty());
        assert_eq!(fake.receives.get(), RX_FRAMES_PER_TICK);
    }

    #[test]
    fn invalid_frames_are_dropped_and_counted() {
        let fake = Fake::new();
        fake.queue.borrow_mut().push_back(pictorus_can_frame_t {
            id: 0x800,
            extended: false,
            dlc: 1,
            data: [0; 8],
        });
        fake.queue.borrow_mut().push_back(pictorus_can_frame_t {
            id: 1,
            extended: false,
            dlc: 9,
            data: [0; 8],
        });
        fake.push(1, false, &[5]);
        let mut bus = bus(&fake);

        assert_eq!(bus.input(&rx_params(std_id(1)), &Clock), [5]);
        assert_eq!(bus.status().error_count, 2);
        assert_eq!(bus.status().last_error, PICTORUS_ERR_INVALID_ARG);
    }

    #[test]
    fn receive_failure_is_recorded_and_stops_the_drain() {
        let fake = Fake::new();
        fake.push(1, false, &[1]);
        fake.receive_result.set(PICTORUS_ERR_PERIPHERAL);
        let mut bus = bus(&fake);

        assert!(bus.input(&rx_params(std_id(1)), &Clock).is_empty());
        assert_eq!(fake.receives.get(), 1);
        assert_eq!(bus.status().error_count, 1);
    }

    #[test]
    fn transmit_builds_a_frame() {
        let fake = Fake::new();
        let mut bus = bus(&fake);
        bus.output(
            &CanTransmitBlockParams::new(ext_id(0x1234)),
            &Clock,
            &[1, 2, 3],
        );

        let sent = fake.sent.borrow();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].id, 0x1234);
        assert!(sent[0].extended);
        assert_eq!(sent[0].data(), [1, 2, 3]);
        assert_eq!(bus.status(), Default::default());
    }

    #[test]
    fn transmit_of_an_oversized_payload_is_skipped() {
        let fake = Fake::new();
        let mut bus = bus(&fake);
        bus.output(&CanTransmitBlockParams::new(std_id(1)), &Clock, &[0; 9]);
        assert!(fake.sent.borrow().is_empty());
        assert_eq!(bus.status(), Default::default());
    }

    #[test]
    fn transmit_failure_is_recorded() {
        let fake = Fake::new();
        fake.transmit_result.set(PICTORUS_ERR_PERIPHERAL);
        let mut bus = bus(&fake);
        bus.output(&CanTransmitBlockParams::new(std_id(1)), &Clock, &[1]);
        assert_eq!(bus.status().error_count, 1);
    }

    #[test]
    fn missing_functions_are_counted_failures() {
        let mut bus = unsafe {
            CanBus::new(pictorus_can_t {
                ctx: core::ptr::null_mut(),
                receive: None,
                transmit: None,
            })
        };
        bus.input(&rx_params(std_id(1)), &Clock);
        bus.output(&CanTransmitBlockParams::new(std_id(1)), &Clock, &[1]);
        assert_eq!(bus.status().error_count, 2);
    }
}
