use core::ffi::c_void;

use pictorus_blocks::{SpiReceiveBlockParams, SpiTransmitBlockParams};
use pictorus_internal::protocols::Flush;
use pictorus_traits::{ByteSliceSignal, InputBlock, ModelClock, OutputBlock, PassBy};

use super::buffer::ByteBuffer;
use super::{Peripheral, pictorus_peripheral_status_t};
use crate::status::{PICTORUS_ERR_PERIPHERAL, pictorus_status_t};

/// Mirror of `pictorus_spi_t` in `pictorus_ffi.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct pictorus_spi_t {
    pub ctx: *mut c_void,
    pub read: Option<
        unsafe extern "C" fn(ctx: *mut c_void, rx: *mut u8, len: usize) -> pictorus_status_t,
    >,
    pub write: Option<
        unsafe extern "C" fn(ctx: *mut c_void, tx: *const u8, len: usize) -> pictorus_status_t,
    >,
}

/// An SPI device backed by C.
///
/// Chip select belongs to C: each call is one complete chip-select framed transaction.
///
/// Reads are cached so that every block reading the device in a tick sees the same data, and C
/// is asked at most once per tick, even if that read fails.
pub struct SpiBus {
    handle: pictorus_spi_t,
    cache: ByteBuffer,
    cache_stale: bool,
    status: pictorus_peripheral_status_t,
}

impl SpiBus {
    /// # Safety
    ///
    /// `handle.ctx` must remain valid for as long as the adapter lives, and the functions in
    /// `handle`, if set, must be sound to call with it.
    pub unsafe fn new(handle: pictorus_spi_t) -> Self {
        Self {
            handle,
            cache: ByteBuffer::new(),
            cache_stale: true,
            status: Default::default(),
        }
    }
}

impl Peripheral for SpiBus {
    fn status(&self) -> pictorus_peripheral_status_t {
        self.status
    }
}

impl InputBlock for SpiBus {
    type Output = ByteSliceSignal;
    type Parameters = SpiReceiveBlockParams;

    fn input(
        &mut self,
        parameters: &Self::Parameters,
        _model_clock: &dyn ModelClock,
    ) -> PassBy<'_, Self::Output> {
        if !self.cache_stale {
            return self.cache.as_slice();
        }
        self.cache_stale = false;

        if parameters.read_bytes == 0 {
            return &[];
        }

        let handle = self.handle;
        let rx = self.cache.prepare(parameters.read_bytes);
        let status = match handle.read {
            // SAFETY: guaranteed by the contract of `new`, and `rx` is valid for its length for
            // the duration of the call.
            Some(read) => unsafe { read(handle.ctx, rx.as_mut_ptr(), rx.len()) },
            None => PICTORUS_ERR_PERIPHERAL,
        };
        if !self.status.record(status) {
            self.cache.clear();
        }
        self.cache.as_slice()
    }
}

impl OutputBlock for SpiBus {
    type Inputs = ByteSliceSignal;
    type Parameters = SpiTransmitBlockParams;

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

impl Flush for SpiBus {
    fn flush(&mut self) {
        self.cache_stale = true;
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peripherals::test_util::Clock;
    use crate::status::PICTORUS_OK;
    use core::cell::{Cell, RefCell};
    use std::vec::Vec;

    // The tests change the fake while the adapter holds a pointer to it, hence the cells.
    struct Fake {
        data: RefCell<Vec<u8>>,
        result: Cell<pictorus_status_t>,
        reads: Cell<usize>,
        written: RefCell<Vec<Vec<u8>>>,
    }

    impl Fake {
        fn new(data: &[u8]) -> Self {
            Self {
                data: RefCell::new(data.to_vec()),
                result: Cell::new(PICTORUS_OK),
                reads: Cell::new(0),
                written: RefCell::new(Vec::new()),
            }
        }
    }

    unsafe extern "C" fn read(ctx: *mut c_void, rx: *mut u8, len: usize) -> pictorus_status_t {
        let fake = unsafe { &*(ctx as *const Fake) };
        fake.reads.set(fake.reads.get() + 1);
        if fake.result.get() == PICTORUS_OK {
            let data = fake.data.borrow();
            unsafe { core::ptr::copy_nonoverlapping(data.as_ptr(), rx, len.min(data.len())) };
        }
        fake.result.get()
    }

    unsafe extern "C" fn write(ctx: *mut c_void, tx: *const u8, len: usize) -> pictorus_status_t {
        let fake = unsafe { &*(ctx as *const Fake) };
        fake.written
            .borrow_mut()
            .push(unsafe { core::slice::from_raw_parts(tx, len) }.to_vec());
        fake.result.get()
    }

    fn bus(fake: &Fake) -> SpiBus {
        unsafe {
            SpiBus::new(pictorus_spi_t {
                ctx: fake as *const Fake as *mut c_void,
                read: Some(read),
                write: Some(write),
            })
        }
    }

    #[test]
    fn reads_once_per_tick() {
        let fake = Fake::new(&[5, 6, 7]);
        let mut bus = bus(&fake);
        let params = SpiReceiveBlockParams::new(3.0, 0.0);

        assert_eq!(bus.input(&params, &Clock), [5, 6, 7]);
        assert_eq!(bus.input(&params, &Clock), [5, 6, 7]);
        assert_eq!(fake.reads.get(), 1);

        bus.flush();
        bus.input(&params, &Clock);
        assert_eq!(fake.reads.get(), 2);
    }

    #[test]
    fn reads_of_any_size_are_supported() {
        let fake = Fake::new(&[7; 4096]);
        let mut bus = bus(&fake);
        let params = SpiReceiveBlockParams::new(4096.0, 0.0);
        assert_eq!(bus.input(&params, &Clock).len(), 4096);
    }

    #[test]
    fn failed_read_yields_no_data_and_is_not_retried_within_tick() {
        let fake = Fake::new(&[1, 2]);
        fake.result.set(PICTORUS_ERR_PERIPHERAL);
        let mut bus = bus(&fake);
        let params = SpiReceiveBlockParams::new(2.0, 0.0);

        assert!(bus.input(&params, &Clock).is_empty());
        assert!(bus.input(&params, &Clock).is_empty());
        assert_eq!(fake.reads.get(), 1);
        assert_eq!(bus.status().error_count, 1);
    }

    #[test]
    fn zero_length_read_does_not_call_c() {
        let fake = Fake::new(&[]);
        let mut bus = bus(&fake);
        assert!(
            bus.input(&SpiReceiveBlockParams::new(0.0, 0.0), &Clock)
                .is_empty()
        );
        assert_eq!(fake.reads.get(), 0);
    }

    #[test]
    fn write_passes_data_through() {
        let fake = Fake::new(&[]);
        let mut bus = bus(&fake);
        let params = SpiTransmitBlockParams::new();
        bus.output(&params, &Clock, &[0xAA, 0xBB]);
        bus.output(&params, &Clock, &[]);
        assert_eq!(*fake.written.borrow(), [std::vec![0xAA, 0xBB], std::vec![]]);
        assert_eq!(bus.status(), Default::default());
    }

    #[test]
    fn missing_functions_are_counted_failures() {
        let mut bus = unsafe {
            SpiBus::new(pictorus_spi_t {
                ctx: core::ptr::null_mut(),
                read: None,
                write: None,
            })
        };
        bus.input(&SpiReceiveBlockParams::new(1.0, 0.0), &Clock);
        bus.output(&SpiTransmitBlockParams::new(), &Clock, &[1]);
        assert_eq!(bus.status().error_count, 2);
    }
}
