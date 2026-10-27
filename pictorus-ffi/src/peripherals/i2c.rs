use core::ffi::c_void;

use pictorus_blocks::{I2cInputBlockParams, I2cOutputBlockParams};
use pictorus_traits::{ByteSliceSignal, InputBlock, ModelClock, OutputBlock, PassBy};

use super::buffer::ByteBuffer;
use super::{Peripheral, pictorus_peripheral_status_t};
use crate::status::{PICTORUS_ERR_PERIPHERAL, pictorus_status_t};

/// Mirror of `pictorus_i2c_t` in `pictorus_ffi.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct pictorus_i2c_t {
    pub ctx: *mut c_void,
    pub write_read: Option<
        unsafe extern "C" fn(
            ctx: *mut c_void,
            addr: u8,
            wr: *const u8,
            wr_len: usize,
            rd: *mut u8,
            rd_len: usize,
        ) -> pictorus_status_t,
    >,
    pub write: Option<
        unsafe extern "C" fn(
            ctx: *mut c_void,
            addr: u8,
            wr: *const u8,
            wr_len: usize,
        ) -> pictorus_status_t,
    >,
}

/// An I2C bus backed by C.
///
/// An input block means "write the command byte, then read `read_bytes` bytes", and an output
/// block means "write the command byte followed by the data". Both are expressed to C as the
/// plain bus operations, so C needs to know nothing about blocks.
///
/// Reads are not cached from tick to tick: each input block performs its own transaction, since
/// several devices can share a bus. A failed read yields no data, so the block's staleness
/// tracking sees it as a missed update.
pub struct I2cBus {
    handle: pictorus_i2c_t,
    rx: ByteBuffer,
    tx: ByteBuffer,
    status: pictorus_peripheral_status_t,
}

impl I2cBus {
    /// # Safety
    ///
    /// `handle.ctx` must remain valid for as long as the adapter lives, and the functions in
    /// `handle`, if set, must be sound to call with it.
    pub unsafe fn new(handle: pictorus_i2c_t) -> Self {
        Self {
            handle,
            rx: ByteBuffer::new(),
            tx: ByteBuffer::new(),
            status: Default::default(),
        }
    }
}

impl Peripheral for I2cBus {
    fn status(&self) -> pictorus_peripheral_status_t {
        self.status
    }
}

impl InputBlock for I2cBus {
    type Output = ByteSliceSignal;
    type Parameters = I2cInputBlockParams;

    fn input(
        &mut self,
        parameters: &Self::Parameters,
        _model_clock: &dyn ModelClock,
    ) -> PassBy<'_, Self::Output> {
        let handle = self.handle;
        let rd = self.rx.prepare(parameters.read_bytes);

        let command = [parameters.command];
        let status = match handle.write_read {
            // SAFETY: guaranteed by the contract of `new`, and both buffers are valid for the
            // lengths passed for the duration of the call.
            Some(write_read) => unsafe {
                write_read(
                    handle.ctx,
                    parameters.address,
                    command.as_ptr(),
                    command.len(),
                    rd.as_mut_ptr(),
                    rd.len(),
                )
            },
            None => PICTORUS_ERR_PERIPHERAL,
        };
        if !self.status.record(status) {
            self.rx.clear();
        }
        self.rx.as_slice()
    }
}

impl OutputBlock for I2cBus {
    type Inputs = ByteSliceSignal;
    type Parameters = I2cOutputBlockParams;

    fn output(
        &mut self,
        parameters: &Self::Parameters,
        _model_clock: &dyn ModelClock,
        inputs: PassBy<'_, Self::Inputs>,
    ) {
        self.tx.set_concat(&[&[parameters.command], inputs]);

        let tx = self.tx.as_slice();
        let status = match self.handle.write {
            // SAFETY: guaranteed by the contract of `new`, and `tx` is valid for its length for
            // the duration of the call.
            Some(write) => unsafe {
                write(self.handle.ctx, parameters.address, tx.as_ptr(), tx.len())
            },
            None => PICTORUS_ERR_PERIPHERAL,
        };
        self.status.record(status);
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
        // What a read returns, and what each call was given.
        read_data: RefCell<Vec<u8>>,
        result: Cell<pictorus_status_t>,
        write_reads: RefCell<Vec<(u8, Vec<u8>, usize)>>,
        writes: RefCell<Vec<(u8, Vec<u8>)>>,
    }

    impl Fake {
        fn new(read_data: &[u8]) -> Self {
            Self {
                read_data: RefCell::new(read_data.to_vec()),
                result: Cell::new(PICTORUS_OK),
                write_reads: RefCell::new(Vec::new()),
                writes: RefCell::new(Vec::new()),
            }
        }
    }

    unsafe extern "C" fn write_read(
        ctx: *mut c_void,
        addr: u8,
        wr: *const u8,
        wr_len: usize,
        rd: *mut u8,
        rd_len: usize,
    ) -> pictorus_status_t {
        let fake = unsafe { &*(ctx as *const Fake) };
        let wr = unsafe { core::slice::from_raw_parts(wr, wr_len) }.to_vec();
        fake.write_reads.borrow_mut().push((addr, wr, rd_len));
        if fake.result.get() == PICTORUS_OK {
            let data = fake.read_data.borrow();
            unsafe { core::ptr::copy_nonoverlapping(data.as_ptr(), rd, rd_len.min(data.len())) };
        }
        fake.result.get()
    }

    unsafe extern "C" fn write(
        ctx: *mut c_void,
        addr: u8,
        wr: *const u8,
        wr_len: usize,
    ) -> pictorus_status_t {
        let fake = unsafe { &*(ctx as *const Fake) };
        let wr = unsafe { core::slice::from_raw_parts(wr, wr_len) }.to_vec();
        fake.writes.borrow_mut().push((addr, wr));
        fake.result.get()
    }

    fn bus(fake: &Fake) -> I2cBus {
        unsafe {
            I2cBus::new(pictorus_i2c_t {
                ctx: fake as *const Fake as *mut c_void,
                write_read: Some(write_read),
                write: Some(write),
            })
        }
    }

    #[test]
    fn input_writes_command_then_reads() {
        let fake = Fake::new(&[0xAB, 0xCD]);
        let mut bus = bus(&fake);
        let params = I2cInputBlockParams::new(104.0, 117.0, 2.0, 0.0);

        assert_eq!(bus.input(&params, &Clock), [0xAB, 0xCD]);
        assert_eq!(*fake.write_reads.borrow(), [(104, std::vec![117], 2)]);
        assert_eq!(bus.status(), Default::default());
    }

    #[test]
    fn reads_are_not_cached() {
        let fake = Fake::new(&[1]);
        let mut bus = bus(&fake);
        let params = I2cInputBlockParams::new(1.0, 2.0, 1.0, 0.0);
        bus.input(&params, &Clock);
        bus.input(&params, &Clock);
        assert_eq!(fake.write_reads.borrow().len(), 2);
    }

    #[test]
    fn reads_of_any_size_are_supported() {
        let fake = Fake::new(&[7; 4096]);
        let mut bus = bus(&fake);
        let params = I2cInputBlockParams::new(1.0, 2.0, 4096.0, 0.0);
        assert_eq!(bus.input(&params, &Clock).len(), 4096);
    }

    #[test]
    fn failed_read_yields_no_data() {
        let fake = Fake::new(&[1, 2]);
        let mut bus = bus(&fake);
        let params = I2cInputBlockParams::new(1.0, 2.0, 2.0, 0.0);
        assert_eq!(bus.input(&params, &Clock).len(), 2);

        fake.result.set(PICTORUS_ERR_PERIPHERAL);
        assert!(bus.input(&params, &Clock).is_empty());
        assert_eq!(bus.status().error_count, 1);
    }

    #[test]
    fn output_prepends_command() {
        let fake = Fake::new(&[]);
        let mut bus = bus(&fake);
        bus.output(
            &I2cOutputBlockParams::new(0x50 as f64, 0x6B as f64),
            &Clock,
            &[1, 2],
        );
        assert_eq!(*fake.writes.borrow(), [(0x50, std::vec![0x6B, 1, 2])]);
    }

    #[test]
    fn output_failure_is_recorded() {
        let fake = Fake::new(&[]);
        fake.result.set(PICTORUS_ERR_PERIPHERAL);
        let mut bus = bus(&fake);
        bus.output(&I2cOutputBlockParams::new(1.0, 2.0), &Clock, &[1]);
        assert_eq!(bus.status().error_count, 1);
    }

    #[test]
    fn missing_functions_are_counted_failures() {
        let mut bus = unsafe {
            I2cBus::new(pictorus_i2c_t {
                ctx: core::ptr::null_mut(),
                write_read: None,
                write: None,
            })
        };
        bus.input(&I2cInputBlockParams::new(1.0, 2.0, 1.0, 0.0), &Clock);
        bus.output(&I2cOutputBlockParams::new(1.0, 2.0), &Clock, &[1]);
        assert_eq!(bus.status().error_count, 2);
    }
}
