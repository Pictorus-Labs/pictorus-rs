use core::ffi::c_void;

use pictorus_blocks::PwmBlockParams;
use pictorus_traits::{ModelClock, OutputBlock, PassBy};

use super::{Peripheral, pictorus_peripheral_status_t};
use crate::status::{PICTORUS_ERR_PERIPHERAL, pictorus_status_t};

/// Mirror of `pictorus_pwm_t` in `pictorus_ffi.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct pictorus_pwm_t {
    pub ctx: *mut c_void,
    pub set: Option<
        unsafe extern "C" fn(
            ctx: *mut c_void,
            frequency_hz: f64,
            duty_ch1: f64,
            duty_ch2: f64,
        ) -> pictorus_status_t,
    >,
}

/// A two channel PWM output backed by C.
///
/// Reconfiguring a timer is not free on most parts, so C is only called when the model's values
/// differ from the last ones C accepted. A call that fails is retried on the next tick.
///
/// The values are passed through as the model produced them: duty cycles are already within
/// `[0, 1]`, and the frequency is non-negative and may be zero.
pub struct PwmOutput {
    handle: pictorus_pwm_t,
    applied: Option<(f64, f64, f64)>,
    status: pictorus_peripheral_status_t,
}

impl PwmOutput {
    /// # Safety
    ///
    /// `handle.ctx` must remain valid for as long as the adapter lives, and `handle.set`, if
    /// set, must be sound to call with it.
    pub unsafe fn new(handle: pictorus_pwm_t) -> Self {
        Self {
            handle,
            applied: None,
            status: Default::default(),
        }
    }
}

impl Peripheral for PwmOutput {
    fn status(&self) -> pictorus_peripheral_status_t {
        self.status
    }
}

impl OutputBlock for PwmOutput {
    type Inputs = (f64, f64, f64); // (Frequency, Duty Cycle Ch1, Duty Cycle Ch2)
    type Parameters = PwmBlockParams;

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _model_clock: &dyn ModelClock,
        inputs: PassBy<'_, Self::Inputs>,
    ) {
        if self.applied == Some(inputs) {
            return;
        }

        let (frequency_hz, duty_ch1, duty_ch2) = inputs;
        let status = match self.handle.set {
            // SAFETY: guaranteed by the contract of `new`.
            Some(set) => unsafe { set(self.handle.ctx, frequency_hz, duty_ch1, duty_ch2) },
            None => PICTORUS_ERR_PERIPHERAL,
        };
        if self.status.record(status) {
            self.applied = Some(inputs);
        }
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
        calls: RefCell<Vec<(f64, f64, f64)>>,
        result: Cell<pictorus_status_t>,
    }

    unsafe extern "C" fn set(
        ctx: *mut c_void,
        frequency_hz: f64,
        duty_ch1: f64,
        duty_ch2: f64,
    ) -> pictorus_status_t {
        let fake = unsafe { &*(ctx as *const Fake) };
        fake.calls
            .borrow_mut()
            .push((frequency_hz, duty_ch1, duty_ch2));
        fake.result.get()
    }

    fn fake(result: pictorus_status_t) -> Fake {
        Fake {
            calls: RefCell::new(Vec::new()),
            result: Cell::new(result),
        }
    }

    fn pwm(fake: &Fake) -> PwmOutput {
        unsafe {
            PwmOutput::new(pictorus_pwm_t {
                ctx: fake as *const Fake as *mut c_void,
                set: Some(set),
            })
        }
    }

    #[test]
    fn only_calls_c_when_values_change() {
        let fake = fake(PICTORUS_OK);
        let mut pwm = pwm(&fake);
        let params = PwmBlockParams::new();

        pwm.output(&params, &Clock, (1000.0, 0.25, 0.75));
        pwm.output(&params, &Clock, (1000.0, 0.25, 0.75));
        assert_eq!(*fake.calls.borrow(), [(1000.0, 0.25, 0.75)]);

        pwm.output(&params, &Clock, (1000.0, 0.5, 0.75));
        assert_eq!(
            *fake.calls.borrow(),
            [(1000.0, 0.25, 0.75), (1000.0, 0.5, 0.75)]
        );
    }

    #[test]
    fn failed_call_is_retried_next_tick() {
        let fake = fake(PICTORUS_ERR_PERIPHERAL);
        let mut pwm = pwm(&fake);
        let params = PwmBlockParams::new();

        pwm.output(&params, &Clock, (500.0, 0.1, 0.2));
        assert_eq!(pwm.status().error_count, 1);

        fake.result.set(PICTORUS_OK);
        pwm.output(&params, &Clock, (500.0, 0.1, 0.2));
        assert_eq!(fake.calls.borrow().len(), 2);
        assert_eq!(pwm.status().error_count, 1);

        pwm.output(&params, &Clock, (500.0, 0.1, 0.2));
        assert_eq!(fake.calls.borrow().len(), 2);
    }

    #[test]
    fn missing_function_is_a_counted_failure() {
        let mut pwm = unsafe {
            PwmOutput::new(pictorus_pwm_t {
                ctx: core::ptr::null_mut(),
                set: None,
            })
        };
        pwm.output(&PwmBlockParams::new(), &Clock, (1.0, 0.0, 0.0));
        assert_eq!(pwm.status().error_count, 1);
    }
}
