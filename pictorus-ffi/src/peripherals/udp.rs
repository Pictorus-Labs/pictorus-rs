use core::ffi::{c_char, c_void};

use pictorus_blocks::{UdpReceiveBlockParams, UdpTransmitBlockParams};
use pictorus_internal::protocols::Flush;
use pictorus_traits::{ByteSliceSignal, InputBlock, ModelClock, OutputBlock, PassBy};

use super::buffer::{ByteBuffer, RECEIVE_BUFFER_BYTES, Received};
use super::{Peripheral, pictorus_peripheral_status_t};
use crate::status::{PICTORUS_ERR_INVALID_ARG, PICTORUS_ERR_PERIPHERAL, pictorus_status_t};

// TODO: Broaden UDP support to `alloc` instead of just `std`
// Traditionally, this has always been a `std` only block and protocol for us. But this was
// only because we only had UDP prototocol implmentations for targets with an OS. With this interface
// there is no reason we can't broaden UDP support to `alloc`. If a user has a udp stack on their
// target, they can implement the C interface and use it with this adapter.

/// Datagrams drained from C per tick, at most.
const MAX_DATAGRAMS_PER_TICK: usize = 256;

/// Mirror of `pictorus_udp_t` in `pictorus_ffi.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct pictorus_udp_t {
    pub ctx: *mut c_void,
    pub recv: Option<
        unsafe extern "C" fn(
            ctx: *mut c_void,
            buf: *mut u8,
            cap: usize,
            len: *mut usize,
        ) -> pictorus_status_t,
    >,
    pub send: Option<
        unsafe extern "C" fn(
            ctx: *mut c_void,
            destination: *const c_char,
            buf: *const u8,
            len: usize,
        ) -> pictorus_status_t,
    >,
}

/// A UDP socket backed by C.
///
/// Binding to the address is up to C, which gets it from the app's peripheral configuration.
/// Each receive drains the datagrams C has queued and keeps the most recent, which every block
/// reading the socket in that tick then sees. A datagram larger than [`RECEIVE_BUFFER_BYTES`]
/// is truncated by C. Transmit blocks name a destination of their own, which is passed to C as
/// a NUL-terminated string.
pub struct UdpPort {
    handle: pictorus_udp_t,
    cache: ByteBuffer,
    scratch: ByteBuffer,
    destination: ByteBuffer,
    cache_stale: bool,
    status: pictorus_peripheral_status_t,
}

impl UdpPort {
    /// # Safety
    ///
    /// `handle.ctx` must remain valid for as long as the adapter lives, and the functions in
    /// `handle`, if set, must be sound to call with it.
    pub unsafe fn new(handle: pictorus_udp_t) -> Self {
        Self {
            handle,
            cache: ByteBuffer::new(),
            scratch: ByteBuffer::new(),
            destination: ByteBuffer::new(),
            cache_stale: true,
            status: Default::default(),
        }
    }

    fn drain_receive_queue(&mut self) {
        let handle = self.handle;
        for _ in 0..MAX_DATAGRAMS_PER_TICK {
            let received =
                self.scratch
                    .receive(&mut self.status, RECEIVE_BUFFER_BYTES, |buf, cap, len| {
                        match handle.recv {
                            // SAFETY: guaranteed by the contract of `new`, and the buffer is
                            // valid for `cap` bytes for the duration of the call.
                            Some(recv) => unsafe { recv(handle.ctx, buf, cap, len) },
                            None => PICTORUS_ERR_PERIPHERAL,
                        }
                    });
            match received {
                // Only the most recent datagram is kept.
                Received::Data => core::mem::swap(&mut self.cache, &mut self.scratch),
                Received::Empty | Received::Failed => break,
            }
        }
    }
}

impl Peripheral for UdpPort {
    fn status(&self) -> pictorus_peripheral_status_t {
        self.status
    }
}

impl InputBlock for UdpPort {
    type Output = ByteSliceSignal;
    type Parameters = UdpReceiveBlockParams;

    fn input(
        &mut self,
        _parameters: &Self::Parameters,
        _model_clock: &dyn ModelClock,
    ) -> PassBy<'_, Self::Output> {
        if self.cache_stale {
            self.cache_stale = false;
            self.drain_receive_queue();
        }
        self.cache.as_slice()
    }
}

impl OutputBlock for UdpPort {
    type Inputs = ByteSliceSignal;
    type Parameters = UdpTransmitBlockParams;

    fn output(
        &mut self,
        parameters: &Self::Parameters,
        _model_clock: &dyn ModelClock,
        inputs: PassBy<'_, Self::Inputs>,
    ) {
        let destination = parameters.destination().as_bytes();
        if destination.contains(&0) {
            self.status.record(PICTORUS_ERR_INVALID_ARG);
            return;
        }
        self.destination.set_concat(&[destination, &[0]]);

        let status = match self.handle.send {
            // SAFETY: guaranteed by the contract of `new`. The destination is NUL-terminated,
            // and both it and `inputs` are valid for the duration of the call.
            Some(send) => unsafe {
                send(
                    self.handle.ctx,
                    self.destination.as_slice().as_ptr().cast(),
                    inputs.as_ptr(),
                    inputs.len(),
                )
            },
            None => PICTORUS_ERR_PERIPHERAL,
        };
        self.status.record(status);
    }
}

impl Flush for UdpPort {
    fn flush(&mut self) {
        self.cache_stale = true;
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peripherals::test_util::Clock;
    use crate::status::{PICTORUS_EMPTY, PICTORUS_OK};
    use core::cell::{Cell, RefCell};
    use core::ffi::CStr;
    use std::collections::VecDeque;
    use std::string::String;
    use std::vec::Vec;

    // The tests change the fake while the adapter holds a pointer to it, hence the cells.
    struct Fake {
        queue: RefCell<VecDeque<Vec<u8>>>,
        recv_result: Cell<pictorus_status_t>,
        recvs: Cell<usize>,
        offered: Cell<usize>,
        sent: RefCell<Vec<(String, Vec<u8>)>>,
        send_result: Cell<pictorus_status_t>,
    }

    impl Fake {
        fn new() -> Self {
            Self {
                queue: RefCell::new(VecDeque::new()),
                recv_result: Cell::new(PICTORUS_OK),
                recvs: Cell::new(0),
                offered: Cell::new(0),
                sent: RefCell::new(Vec::new()),
                send_result: Cell::new(PICTORUS_OK),
            }
        }
    }

    unsafe extern "C" fn recv(
        ctx: *mut c_void,
        buf: *mut u8,
        cap: usize,
        len: *mut usize,
    ) -> pictorus_status_t {
        let fake = unsafe { &*(ctx as *const Fake) };
        fake.recvs.set(fake.recvs.get() + 1);
        fake.offered.set(cap);
        if fake.recv_result.get() != PICTORUS_OK {
            return fake.recv_result.get();
        }
        match fake.queue.borrow_mut().pop_front() {
            Some(datagram) => {
                let n = datagram.len().min(cap);
                unsafe { core::ptr::copy_nonoverlapping(datagram.as_ptr(), buf, n) };
                unsafe { *len = n };
                PICTORUS_OK
            }
            None => PICTORUS_EMPTY,
        }
    }

    unsafe extern "C" fn send(
        ctx: *mut c_void,
        destination: *const c_char,
        buf: *const u8,
        len: usize,
    ) -> pictorus_status_t {
        let fake = unsafe { &*(ctx as *const Fake) };
        let destination = unsafe { CStr::from_ptr(destination) }.to_str().unwrap();
        fake.sent.borrow_mut().push((
            destination.into(),
            unsafe { core::slice::from_raw_parts(buf, len) }.to_vec(),
        ));
        fake.send_result.get()
    }

    fn port(fake: &Fake) -> UdpPort {
        unsafe {
            UdpPort::new(pictorus_udp_t {
                ctx: fake as *const Fake as *mut c_void,
                recv: Some(recv),
                send: Some(send),
            })
        }
    }

    #[test]
    fn keeps_only_the_most_recent_datagram() {
        let fake = Fake::new();
        fake.queue.borrow_mut().push_back(b"one".to_vec());
        fake.queue.borrow_mut().push_back(b"two".to_vec());
        let mut port = port(&fake);
        let params = UdpReceiveBlockParams::new(0.0);

        assert_eq!(port.input(&params, &Clock), b"two");
        // Both datagrams plus the call that reported the queue empty.
        assert_eq!(fake.recvs.get(), 3);
        assert_eq!(fake.offered.get(), RECEIVE_BUFFER_BYTES);
        assert_eq!(port.input(&params, &Clock), b"two");
        assert_eq!(fake.recvs.get(), 3);
    }

    #[test]
    fn nothing_queued_is_not_an_error() {
        let fake = Fake::new();
        let mut port = port(&fake);
        assert!(
            port.input(&UdpReceiveBlockParams::new(0.0), &Clock)
                .is_empty()
        );
        assert_eq!(port.status(), Default::default());
    }

    #[test]
    fn flush_makes_the_next_tick_receive_again() {
        let fake = Fake::new();
        fake.queue.borrow_mut().push_back(b"a".to_vec());
        let mut port = port(&fake);
        let params = UdpReceiveBlockParams::new(0.0);
        assert_eq!(port.input(&params, &Clock), b"a");

        port.flush();
        assert!(port.input(&params, &Clock).is_empty());
        fake.queue.borrow_mut().push_back(b"b".to_vec());
        port.flush();
        assert_eq!(port.input(&params, &Clock), b"b");
    }

    #[test]
    fn failed_receive_is_recorded() {
        let fake = Fake::new();
        fake.queue.borrow_mut().push_back(b"a".to_vec());
        fake.recv_result.set(PICTORUS_ERR_PERIPHERAL);
        let mut port = port(&fake);

        assert!(
            port.input(&UdpReceiveBlockParams::new(0.0), &Clock)
                .is_empty()
        );
        assert_eq!(port.status().error_count, 1);
    }

    #[test]
    fn send_passes_a_nul_terminated_destination() {
        let fake = Fake::new();
        let mut port = port(&fake);
        port.output(
            &UdpTransmitBlockParams::new(b"10.0.0.2:9000"),
            &Clock,
            &[1, 2],
        );
        assert_eq!(
            *fake.sent.borrow(),
            [("10.0.0.2:9000".into(), std::vec![1, 2])]
        );
        assert_eq!(port.status(), Default::default());
    }

    #[test]
    fn destination_with_a_nul_is_rejected() {
        let fake = Fake::new();
        let mut port = port(&fake);
        port.output(
            &UdpTransmitBlockParams::new(b"10.0.0.2\0:9000"),
            &Clock,
            &[1],
        );
        assert!(fake.sent.borrow().is_empty());
        assert_eq!(port.status().last_error, PICTORUS_ERR_INVALID_ARG);
    }

    #[test]
    fn missing_functions_are_counted_failures() {
        let mut port = unsafe {
            UdpPort::new(pictorus_udp_t {
                ctx: core::ptr::null_mut(),
                recv: None,
                send: None,
            })
        };
        port.input(&UdpReceiveBlockParams::new(0.0), &Clock);
        port.output(&UdpTransmitBlockParams::new(b"x:1"), &Clock, &[1]);
        assert_eq!(port.status().error_count, 2);
    }
}
