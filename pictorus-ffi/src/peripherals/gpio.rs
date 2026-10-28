use core::ffi::c_void;

use pictorus_blocks::{GpioInputBlockParams, GpioOutputBlockParams};
use pictorus_traits::{InputBlock, ModelClock, OutputBlock, PassBy};

use super::{Peripheral, pictorus_peripheral_status_t};
use crate::status::{PICTORUS_ERR_PERIPHERAL, pictorus_status_t};

/// Mirror of `pictorus_gpio_in_t` in `pictorus_ffi.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct pictorus_gpio_in_t {
    pub ctx: *mut c_void,
    pub read: Option<unsafe extern "C" fn(ctx: *mut c_void, level: *mut bool) -> pictorus_status_t>,
}

/// Mirror of `pictorus_gpio_out_t` in `pictorus_ffi.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct pictorus_gpio_out_t {
    pub ctx: *mut c_void,
    pub write: Option<unsafe extern "C" fn(ctx: *mut c_void, level: bool) -> pictorus_status_t>,
}

/// A digital input backed by C. Reads as `1.0` when high and `0.0` when low.
pub struct GpioInput {
    handle: pictorus_gpio_in_t,
    /// The last level C read successfully, which is reported again if a read fails. This is not
    /// a per-tick cache: every call reads C afresh.
    last: f64,
    status: pictorus_peripheral_status_t,
}

impl GpioInput {
    /// # Safety
    ///
    /// `handle.ctx` must remain valid for as long as the adapter lives, and `handle.read`, if
    /// set, must be sound to call with it.
    pub unsafe fn new(handle: pictorus_gpio_in_t) -> Self {
        Self {
            handle,
            last: 0.0,
            status: Default::default(),
        }
    }
}

impl Peripheral for GpioInput {
    fn status(&self) -> pictorus_peripheral_status_t {
        self.status
    }
}

impl InputBlock for GpioInput {
    type Output = f64;
    type Parameters = GpioInputBlockParams;

    fn input(
        &mut self,
        _parameters: &Self::Parameters,
        _model_clock: &dyn ModelClock,
    ) -> PassBy<'_, Self::Output> {
        let mut level = false;
        let status = match self.handle.read {
            // SAFETY: guaranteed by the contract of `new`.
            Some(read) => unsafe { read(self.handle.ctx, &mut level) },
            None => PICTORUS_ERR_PERIPHERAL,
        };
        if self.status.record(status) {
            self.last = level.into();
        }
        self.last
    }
}

/// A digital output backed by C.
pub struct GpioOutput {
    handle: pictorus_gpio_out_t,
    status: pictorus_peripheral_status_t,
}

impl GpioOutput {
    /// # Safety
    ///
    /// `handle.ctx` must remain valid for as long as the adapter lives, and `handle.write`, if
    /// set, must be sound to call with it.
    pub unsafe fn new(handle: pictorus_gpio_out_t) -> Self {
        Self {
            handle,
            status: Default::default(),
        }
    }
}

impl Peripheral for GpioOutput {
    fn status(&self) -> pictorus_peripheral_status_t {
        self.status
    }
}

impl OutputBlock for GpioOutput {
    type Inputs = bool;
    type Parameters = GpioOutputBlockParams;

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _model_clock: &dyn ModelClock,
        inputs: PassBy<'_, Self::Inputs>,
    ) {
        let status = match self.handle.write {
            // SAFETY: guaranteed by the contract of `new`.
            Some(write) => unsafe { write(self.handle.ctx, inputs) },
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
    use core::cell::Cell;

    // The tests change the fake while the adapter holds a pointer to it, hence `Cell`.
    struct Fake {
        level: Cell<bool>,
        result: Cell<pictorus_status_t>,
        written: Cell<Option<bool>>,
    }

    impl Fake {
        fn new(level: bool, result: pictorus_status_t) -> Self {
            Self {
                level: Cell::new(level),
                result: Cell::new(result),
                written: Cell::new(None),
            }
        }

        fn ctx(&self) -> *mut c_void {
            self as *const Self as *mut c_void
        }
    }

    unsafe extern "C" fn read(ctx: *mut c_void, level: *mut bool) -> pictorus_status_t {
        let fake = unsafe { &*(ctx as *const Fake) };
        if fake.result.get() == PICTORUS_OK {
            unsafe { *level = fake.level.get() };
        }
        fake.result.get()
    }

    unsafe extern "C" fn write(ctx: *mut c_void, level: bool) -> pictorus_status_t {
        let fake = unsafe { &*(ctx as *const Fake) };
        fake.written.set(Some(level));
        fake.result.get()
    }

    fn input(fake: &Fake) -> GpioInput {
        unsafe {
            GpioInput::new(pictorus_gpio_in_t {
                ctx: fake.ctx(),
                read: Some(read),
            })
        }
    }

    #[test]
    fn input_reports_level_as_float() {
        let fake = Fake::new(true, PICTORUS_OK);
        let mut pin = input(&fake);
        assert_eq!(pin.input(&GpioInputBlockParams::new(), &Clock), 1.0);
        fake.level.set(false);
        assert_eq!(pin.input(&GpioInputBlockParams::new(), &Clock), 0.0);
        assert_eq!(pin.status(), Default::default());
    }

    #[test]
    fn input_keeps_previous_value_on_failure() {
        let fake = Fake::new(true, PICTORUS_OK);
        let mut pin = input(&fake);
        assert_eq!(pin.input(&GpioInputBlockParams::new(), &Clock), 1.0);

        fake.result.set(PICTORUS_ERR_PERIPHERAL);
        fake.level.set(false);
        assert_eq!(pin.input(&GpioInputBlockParams::new(), &Clock), 1.0);
        assert_eq!(pin.status().error_count, 1);
        assert_eq!(pin.status().last_error, PICTORUS_ERR_PERIPHERAL);
    }

    #[test]
    fn missing_function_is_a_counted_failure() {
        let mut pin = unsafe {
            GpioInput::new(pictorus_gpio_in_t {
                ctx: core::ptr::null_mut(),
                read: None,
            })
        };
        assert_eq!(pin.input(&GpioInputBlockParams::new(), &Clock), 0.0);
        assert_eq!(pin.status().error_count, 1);

        let mut out = unsafe {
            GpioOutput::new(pictorus_gpio_out_t {
                ctx: core::ptr::null_mut(),
                write: None,
            })
        };
        out.output(&GpioOutputBlockParams::new(), &Clock, true);
        assert_eq!(out.status().error_count, 1);
    }

    #[test]
    fn output_writes_level_and_records_failure() {
        let fake = Fake::new(false, PICTORUS_OK);
        let mut pin = unsafe {
            GpioOutput::new(pictorus_gpio_out_t {
                ctx: fake.ctx(),
                write: Some(write),
            })
        };
        pin.output(&GpioOutputBlockParams::new(), &Clock, true);
        assert_eq!(fake.written.get(), Some(true));
        assert_eq!(pin.status(), Default::default());

        fake.result.set(PICTORUS_ERR_PERIPHERAL);
        pin.output(&GpioOutputBlockParams::new(), &Clock, false);
        assert_eq!(fake.written.get(), Some(false));
        assert_eq!(pin.status().error_count, 1);
    }
}
