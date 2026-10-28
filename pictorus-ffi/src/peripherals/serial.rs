use core::ffi::c_void;

use pictorus_blocks::{SerialReceiveBlockParams, SerialTransmitBlockParams};
use pictorus_internal::protocols::Flush;
use pictorus_traits::{ByteSliceSignal, InputBlock, ModelClock, OutputBlock, PassBy};

use super::byte_buffer::{ByteBuffer, RECEIVE_BUFFER_BYTES};
use super::{Peripheral, pictorus_peripheral_status_t};
use crate::status::{PICTORUS_ERR_PERIPHERAL, pictorus_status_t};

/// Mirror of `pictorus_serial_t` in `pictorus_ffi.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct pictorus_serial_t {
    pub ctx: *mut c_void,
    pub read: Option<
        unsafe extern "C" fn(
            ctx: *mut c_void,
            buf: *mut u8,
            cap: usize,
            len: *mut usize,
        ) -> pictorus_status_t,
    >,
    pub write: Option<
        unsafe extern "C" fn(ctx: *mut c_void, buf: *const u8, len: usize) -> pictorus_status_t,
    >,
}

/// A serial port backed by C.
///
/// C returns whatever bytes have arrived since the last read, without blocking, up to
/// [`RECEIVE_BUFFER_BYTES`]. Bytes it leaves unread stay with C for a later tick. They are
/// cached so every block reading the port in a tick sees the same bytes, and C is asked at most
/// once per tick. Start and end delimiters are handled by the receive block, not here.
pub struct SerialPort {
    handle: pictorus_serial_t,
    cache: ByteBuffer,
    cache_stale: bool,
    status: pictorus_peripheral_status_t,
}

impl SerialPort {
    /// # Safety
    ///
    /// `handle.ctx` must remain valid for as long as the adapter lives, and the functions in
    /// `handle`, if set, must be sound to call with it.
    pub unsafe fn new(handle: pictorus_serial_t) -> Self {
        Self {
            handle,
            cache: ByteBuffer::new(),
            cache_stale: true,
            status: Default::default(),
        }
    }
}

impl Peripheral for SerialPort {
    fn status(&self) -> pictorus_peripheral_status_t {
        self.status
    }
}

impl InputBlock for SerialPort {
    type Output = ByteSliceSignal;
    type Parameters = SerialReceiveBlockParams;

    fn input(
        &mut self,
        _parameters: &Self::Parameters,
        _model_clock: &dyn ModelClock,
    ) -> PassBy<'_, Self::Output> {
        if self.cache_stale {
            self.cache_stale = false;
            let handle = self.handle;
            self.cache
                .receive(&mut self.status, RECEIVE_BUFFER_BYTES, |buf, cap, len| {
                    match handle.read {
                        // SAFETY: guaranteed by the contract of `new`, and the buffer is valid
                        // for `cap` bytes for the duration of the call.
                        Some(read) => unsafe { read(handle.ctx, buf, cap, len) },
                        None => PICTORUS_ERR_PERIPHERAL,
                    }
                });
        }
        self.cache.as_slice()
    }
}

impl OutputBlock for SerialPort {
    type Inputs = ByteSliceSignal;
    type Parameters = SerialTransmitBlockParams;

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _model_clock: &dyn ModelClock,
        inputs: PassBy<'_, Self::Inputs>,
    ) {
        let status = match self.handle.write {
            // SAFETY: guaranteed by the contract of `new`, and `inputs` is valid for its length
            // for the duration of the call.
            Some(write) => unsafe { write(self.handle.ctx, inputs.as_ptr(), inputs.len()) },
            None => PICTORUS_ERR_PERIPHERAL,
        };
        self.status.record(status);
    }
}

impl Flush for SerialPort {
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
    use std::vec::Vec;

    // The tests change the fake while the adapter holds a pointer to it, hence the cells.
    struct Fake {
        incoming: RefCell<Vec<u8>>,
        result: Cell<pictorus_status_t>,
        reads: Cell<usize>,
        offered: Cell<usize>,
        written: RefCell<Vec<Vec<u8>>>,
    }

    impl Fake {
        fn new(incoming: &[u8]) -> Self {
            Self {
                incoming: RefCell::new(incoming.to_vec()),
                result: Cell::new(PICTORUS_OK),
                reads: Cell::new(0),
                offered: Cell::new(0),
                written: RefCell::new(Vec::new()),
            }
        }
    }

    unsafe extern "C" fn read(
        ctx: *mut c_void,
        buf: *mut u8,
        cap: usize,
        len: *mut usize,
    ) -> pictorus_status_t {
        let fake = unsafe { &*(ctx as *const Fake) };
        fake.reads.set(fake.reads.get() + 1);
        fake.offered.set(cap);
        if fake.result.get() != PICTORUS_OK {
            return fake.result.get();
        }
        let mut incoming = fake.incoming.borrow_mut();
        if incoming.is_empty() {
            return PICTORUS_EMPTY;
        }
        // Hands over as much as fits, like a driver draining its receive ring.
        let n = incoming.len().min(cap);
        unsafe { core::ptr::copy_nonoverlapping(incoming.as_ptr(), buf, n) };
        incoming.drain(..n);
        unsafe { *len = n };
        PICTORUS_OK
    }

    unsafe extern "C" fn write(ctx: *mut c_void, buf: *const u8, len: usize) -> pictorus_status_t {
        let fake = unsafe { &*(ctx as *const Fake) };
        fake.written
            .borrow_mut()
            .push(unsafe { core::slice::from_raw_parts(buf, len) }.to_vec());
        fake.result.get()
    }

    fn port(fake: &Fake) -> SerialPort {
        unsafe {
            SerialPort::new(pictorus_serial_t {
                ctx: fake as *const Fake as *mut c_void,
                read: Some(read),
                write: Some(write),
            })
        }
    }

    fn params() -> SerialReceiveBlockParams {
        SerialReceiveBlockParams::new("", "", 0.0, 0.0)
    }

    #[test]
    fn reads_once_per_tick() {
        let fake = Fake::new(b"abc");
        let mut port = port(&fake);

        assert_eq!(port.input(&params(), &Clock), b"abc");
        fake.incoming.borrow_mut().extend_from_slice(b"def");
        assert_eq!(port.input(&params(), &Clock), b"abc");
        assert_eq!(fake.reads.get(), 1);

        port.flush();
        assert_eq!(port.input(&params(), &Clock), b"def");
        assert_eq!(fake.reads.get(), 2);
    }

    #[test]
    fn offers_a_receive_buffer_and_leaves_the_rest_with_c() {
        let fake = Fake::new(&std::vec![1; RECEIVE_BUFFER_BYTES + 10]);
        let mut port = port(&fake);

        assert_eq!(port.input(&params(), &Clock).len(), RECEIVE_BUFFER_BYTES);
        assert_eq!(fake.offered.get(), RECEIVE_BUFFER_BYTES);
        port.flush();
        assert_eq!(port.input(&params(), &Clock).len(), 10);
    }

    #[test]
    fn no_data_is_not_an_error() {
        let fake = Fake::new(b"");
        let mut port = port(&fake);
        assert!(port.input(&params(), &Clock).is_empty());
        assert_eq!(port.status(), Default::default());
    }

    #[test]
    fn failed_read_yields_no_data_and_is_not_retried_within_tick() {
        let fake = Fake::new(b"abc");
        fake.result.set(PICTORUS_ERR_PERIPHERAL);
        let mut port = port(&fake);

        assert!(port.input(&params(), &Clock).is_empty());
        assert!(port.input(&params(), &Clock).is_empty());
        assert_eq!(fake.reads.get(), 1);
        assert_eq!(port.status().error_count, 1);
    }

    #[test]
    fn write_passes_data_through() {
        let fake = Fake::new(b"");
        let mut port = port(&fake);
        port.output(&SerialTransmitBlockParams::new("", ""), &Clock, b"hi");
        assert_eq!(*fake.written.borrow(), [b"hi".to_vec()]);
    }

    #[test]
    fn missing_functions_are_counted_failures() {
        let mut port = unsafe {
            SerialPort::new(pictorus_serial_t {
                ctx: core::ptr::null_mut(),
                read: None,
                write: None,
            })
        };
        port.input(&params(), &Clock);
        port.output(&SerialTransmitBlockParams::new("", ""), &Clock, b"x");
        assert_eq!(port.status().error_count, 2);
    }
}
