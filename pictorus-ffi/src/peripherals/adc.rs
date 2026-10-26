use core::ffi::c_void;

use pictorus_blocks::AdcBlockParams;
use pictorus_internal::protocols::Flush;
use pictorus_traits::{InputBlock, ModelClock, PassBy};

use super::{Peripheral, pictorus_peripheral_status_t};
use crate::status::{PICTORUS_ERR_PERIPHERAL, pictorus_status_t};

/// Mirror of `pictorus_adc_t` in `pictorus_ffi.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct pictorus_adc_t {
    pub ctx: *mut c_void,
    pub read: Option<unsafe extern "C" fn(ctx: *mut c_void, sample: *mut u16) -> pictorus_status_t>,
}

/// An ADC input backed by C.
///
/// Every block reading the same ADC in a tick sees the same sample, so C is asked at most once
/// per tick. That includes a failed read: it is not retried until the next tick.
pub struct AdcInput {
    handle: pictorus_adc_t,
    value: u16,
    sampled_this_tick: bool,
    status: pictorus_peripheral_status_t,
}

impl AdcInput {
    /// # Safety
    ///
    /// `handle.ctx` must remain valid for as long as the adapter lives, and `handle.read`, if
    /// set, must be sound to call with it.
    pub unsafe fn new(handle: pictorus_adc_t) -> Self {
        Self {
            handle,
            value: 0,
            sampled_this_tick: false,
            status: Default::default(),
        }
    }
}

impl Peripheral for AdcInput {
    fn status(&self) -> pictorus_peripheral_status_t {
        self.status
    }
}

impl InputBlock for AdcInput {
    type Output = u16;
    type Parameters = AdcBlockParams;

    fn input(
        &mut self,
        _parameters: &Self::Parameters,
        _model_clock: &dyn ModelClock,
    ) -> PassBy<'_, Self::Output> {
        if !self.sampled_this_tick {
            self.sampled_this_tick = true;
            let mut sample = 0;
            let status = match self.handle.read {
                // SAFETY: guaranteed by the contract of `new`.
                Some(read) => unsafe { read(self.handle.ctx, &mut sample) },
                None => PICTORUS_ERR_PERIPHERAL,
            };
            if self.status.record(status) {
                self.value = sample;
            }
        }
        self.value
    }
}

impl Flush for AdcInput {
    fn flush(&mut self) {
        self.sampled_this_tick = false;
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
        sample: Cell<u16>,
        result: Cell<pictorus_status_t>,
        calls: Cell<usize>,
    }

    unsafe extern "C" fn read(ctx: *mut c_void, sample: *mut u16) -> pictorus_status_t {
        let fake = unsafe { &*(ctx as *const Fake) };
        fake.calls.set(fake.calls.get() + 1);
        if fake.result.get() == PICTORUS_OK {
            unsafe { *sample = fake.sample.get() };
        }
        fake.result.get()
    }

    fn fake(sample: u16, result: pictorus_status_t) -> Fake {
        Fake {
            sample: Cell::new(sample),
            result: Cell::new(result),
            calls: Cell::new(0),
        }
    }

    fn adc(fake: &Fake) -> AdcInput {
        unsafe {
            AdcInput::new(pictorus_adc_t {
                ctx: fake as *const Fake as *mut c_void,
                read: Some(read),
            })
        }
    }

    #[test]
    fn samples_once_per_tick() {
        let fake = fake(1234, PICTORUS_OK);
        let mut adc = adc(&fake);
        let params = AdcBlockParams::new();

        assert_eq!(adc.input(&params, &Clock), 1234);
        fake.sample.set(99);
        assert_eq!(adc.input(&params, &Clock), 1234);
        assert_eq!(fake.calls.get(), 1);

        adc.flush();
        assert_eq!(adc.input(&params, &Clock), 99);
        assert_eq!(fake.calls.get(), 2);
    }

    #[test]
    fn failed_read_keeps_previous_value_and_is_not_retried_within_tick() {
        let fake = fake(500, PICTORUS_OK);
        let mut adc = adc(&fake);
        let params = AdcBlockParams::new();
        assert_eq!(adc.input(&params, &Clock), 500);
        adc.flush();

        fake.result.set(PICTORUS_ERR_PERIPHERAL);
        fake.sample.set(7);
        assert_eq!(adc.input(&params, &Clock), 500);
        assert_eq!(adc.input(&params, &Clock), 500);
        assert_eq!(fake.calls.get(), 2);
        assert_eq!(adc.status().error_count, 1);
    }
}
