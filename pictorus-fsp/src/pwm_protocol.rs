//! PWM, driven through the timer vtable.
//!
//! A Pictorus PWM block speaks hertz and a duty fraction in 0..1; `timer_api_t`
//! speaks raw counter counts, whose meaning depends on the clock the
//! configurator gave the timer.

use pictorus_blocks::PwmBlockParams;
use pictorus_traits::{ModelClock, OutputBlock, PassBy};
use renesas_fsp_sys::{timer_info_t, timer_instance_t};

use crate::diag::warn_once;
use crate::error::{FspError, Result, check};

/// Compare outputs a GPT or AGT drives: GTIOCA and GTIOCB.
pub const TIMER_OUTPUTS: usize = 2;

/// Output pin selector for `timer_api_t::dutyCycleSet`.
///
/// The interface documents this only as "which output pin to update, see
/// implementation for details". For GPT and AGT it selects between the timer's
/// two compare outputs: 0 is GTIOCA, 1 is GTIOCB.
pub type TimerPin = u32;

pub const GTIOCA: TimerPin = 0;
pub const GTIOCB: TimerPin = 1;

/// A timer driving its [`TIMER_OUTPUTS`] compare outputs.
///
/// The frequency and duty cycles arrive from the model every tick; everything
/// electrical is configured on the FSP timer instance.
#[derive(Debug)]
pub struct FspPwm {
    instance: *const timer_instance_t,
    /// Which timer output each of the model's duty inputs drives. `None`
    /// means the model does not use that output.
    pins: [Option<TimerPin>; TIMER_OUTPUTS],
    /// Counter frequency reported by `infoGet`, cached because it cannot change
    /// without the timer being reconfigured, which only the configurator does.
    clock_frequency: u32,
    /// Last period written, in counts. Tracked so an unchanged frequency does
    /// not re-enter `periodSet` every tick.
    period_counts: u32,
    /// Last duty written per channel, in counts.
    duty_counts: [u32; TIMER_OUTPUTS],
}

impl FspPwm {
    /// Open the timer, start it, and read back its counter frequency.
    ///
    /// # Safety
    ///
    /// `instance` must point at a live `timer_instance_t` from the generated
    /// `ra_gen/hal_data.c` and must outlive this wrapper.
    pub unsafe fn open(
        instance: *const timer_instance_t,
        pins: [Option<TimerPin>; TIMER_OUTPUTS],
    ) -> Result<Self> {
        let mut pwm = Self {
            instance,
            pins,
            clock_frequency: 0,
            period_counts: 0,
            duty_counts: [0; TIMER_OUTPUTS],
        };

        {
            let (instance_ref, api) = pwm.parts()?;
            let open = api.open.ok_or(FspError::UNIMPLEMENTED)?;
            // SAFETY: caller's contract; p_cfg is the generated const cfg.
            check(unsafe { open(instance_ref.p_ctrl, instance_ref.p_cfg) })?;
        }

        let info = match pwm.info() {
            Ok(info) if info.clock_frequency != 0 => info,
            // Every later conversion divides by the clock. Reporting it here
            // names the timer configuration as the cause; letting it through
            // would surface as a division by zero deep in a tick.
            Ok(_) => return pwm.close_after_failure(FspError::INVALID_CLOCK),
            Err(err) => return pwm.close_after_failure(err),
        };

        pwm.clock_frequency = info.clock_frequency;

        let start_result = {
            let (instance_ref, api) = pwm.parts()?;
            match api.start {
                // SAFETY: as above.
                Some(start) => check(unsafe { start(instance_ref.p_ctrl) }),
                None => Err(FspError::UNIMPLEMENTED),
            }
        };
        if let Err(err) = start_result {
            return pwm.close_after_failure(err);
        }

        Ok(pwm)
    }

    /// Undo `open` so a failed construction does not strand the peripheral.
    fn close_after_failure(&self, reason: FspError) -> Result<Self> {
        if let Ok((instance, api)) = self.parts()
            && let Some(close) = api.close
        {
            // SAFETY: open's contract; the instance was opened just above.
            let _ = unsafe { close(instance.p_ctrl) };
        }
        Err(reason)
    }

    fn parts(&self) -> Result<(&timer_instance_t, &renesas_fsp_sys::timer_api_t)> {
        // SAFETY: open's contract.
        let instance = unsafe { self.instance.as_ref() }.ok_or(FspError::NULL_INSTANCE)?;
        let api = unsafe { instance.p_api.as_ref() }.ok_or(FspError::NULL_INSTANCE)?;
        Ok((instance, api))
    }

    fn info(&self) -> Result<timer_info_t> {
        let (instance, api) = self.parts()?;
        let info_get = api.infoGet.ok_or(FspError::UNIMPLEMENTED)?;
        let mut info = core::mem::MaybeUninit::<timer_info_t>::uninit();
        // SAFETY: open's contract; infoGet fills every field of *p_info.
        check(unsafe { info_get(instance.p_ctrl, info.as_mut_ptr()) })?;
        // SAFETY: the call above succeeded, so p_info is initialised.
        Ok(unsafe { info.assume_init() })
    }

    /// Counter counts for one period at `frequency` hertz, or `None` if the
    /// frequency is not a period this timer can express.
    ///
    /// A model can compute any frequency at runtime, including zero, negative
    /// and NaN. Rather than substitute an extreme, an unusable frequency 
    /// leaves the period alone.
    fn period_counts_for(&self, frequency: f64) -> Option<u32> {
        // NaN has to be tested for rather than compared.
        if frequency.is_nan() || frequency <= 0.0 {
            return None;
        }
        let counts = libm::round(self.clock_frequency as f64 / frequency);
        if counts >= u32::MAX as f64 {
            // hold rather than guess at the width.
            None
        } else if counts < 1.0 {
            Some(1)
        } else {
            Some(counts as u32)
        }
    }

    fn duty_counts_for(&self, period_counts: u32, duty: f64) -> u32 {
        if !duty.is_finite() {
            return 0;
        }
        libm::round(duty.clamp(0.0, 1.0) * period_counts as f64) as u32
    }

    fn set_period(&mut self, period_counts: u32) -> Result<()> {
        let (instance, api) = self.parts()?;
        let period_set = api.periodSet.ok_or(FspError::UNIMPLEMENTED)?;
        // SAFETY: open's contract.
        check(unsafe { period_set(instance.p_ctrl, period_counts) })?;
        self.period_counts = period_counts;
        Ok(())
    }

    fn set_duty(&mut self, channel: usize, pin: TimerPin, duty_counts: u32) -> Result<()> {
        let (instance, api) = self.parts()?;
        let duty_set = api.dutyCycleSet.ok_or(FspError::UNIMPLEMENTED)?;
        // SAFETY: open's contract.
        check(unsafe { duty_set(instance.p_ctrl, duty_counts, pin) })?;
        self.duty_counts[channel] = duty_counts;
        Ok(())
    }

    fn apply(&mut self, frequency: f64, duties: [f64; TIMER_OUTPUTS]) -> Result<()> {
        let Some(period_counts) = self.period_counts_for(frequency) else {
            warn_once!(
                "PWM frequency {frequency} Hz is not representable on this timer; \
                 holding the previous period"
            );
            return Ok(());
        };

        // `self.period_counts` is zero until the first successful update.
        let period_changed = period_counts != self.period_counts;

        if period_changed {
            self.set_period(period_counts)?;
        }

        for (channel, &duty) in duties.iter().enumerate() {
            // `None` means the model left this output unused.
            let Some(pin) = self.pins[channel] else {
                continue;
            };
            let wanted = self.duty_counts_for(period_counts, duty);
            if period_changed || wanted != self.duty_counts[channel] {
                self.set_duty(channel, pin, wanted)?;
            }
        }

        Ok(())
    }
}

/// One update drives every output the timer has.
impl OutputBlock for FspPwm {
    /// `(frequency_hz, duty_gtioca, duty_gtiocb)`.
    type Inputs = (f64, f64, f64);
    type Parameters = PwmBlockParams;

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn ModelClock,
        inputs: PassBy<'_, Self::Inputs>,
    ) {
        let (frequency, duty_a, duty_b) = inputs;
        if let Err(err) = self.apply(frequency, [duty_a, duty_b]) {
            warn_once!("PWM update failed: {}", err);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A wrapper for exercising the unit conversion alone.
    ///
    /// The instance pointer is null and stays null: nothing reached from here
    /// dereferences it. The conversions are the part worth testing, because
    /// they are the only arithmetic in the crate whose result is a physical
    /// quantity, and a wrong answer produces a working PWM signal at the wrong
    /// frequency rather than a failure.
    fn pwm(clock_frequency: u32) -> FspPwm {
        FspPwm {
            instance: core::ptr::null(),
            pins: [Some(GTIOCA), Some(GTIOCB)],
            clock_frequency,
            period_counts: 0,
            duty_counts: [0; TIMER_OUTPUTS],
        }
    }

    #[test]
    fn period_is_clock_over_frequency() {
        let pwm = pwm(120_000_000);
        assert_eq!(pwm.period_counts_for(1_000.0), Some(120_000));
        assert_eq!(pwm.period_counts_for(20_000.0), Some(6_000));
    }

    #[test]
    fn period_rounds_rather_than_truncates() {
        // 120 MHz / 7 kHz = 17142.857..., so truncation would cost 0.86 counts
        // and bias every non-dividing frequency high.
        assert_eq!(pwm(120_000_000).period_counts_for(7_000.0), Some(17_143));
    }

    #[test]
    fn nonsense_frequencies_hold_the_previous_period() {
        // A model can compute any of these at runtime, and `output` has no way
        // to refuse. The slowest period is the least surprising answer and
        // keeps the counter arithmetic well defined.
        let pwm = pwm(120_000_000);
        assert_eq!(pwm.period_counts_for(0.0), None);
        assert_eq!(pwm.period_counts_for(-100.0), None);
        assert_eq!(pwm.period_counts_for(f64::NAN), None);
        // Infinity is representable: as fast as the counter goes.
        assert_eq!(pwm.period_counts_for(f64::INFINITY), Some(1));
    }

    #[test]
    fn frequency_above_the_counter_clamps_to_one_count() {
        // Faster than the counter can express; one count is the floor.
        assert_eq!(pwm(1_000).period_counts_for(10_000.0), Some(1));
    }

    /// A frequency slower than a 32-bit counter can express is held, not
    /// saturated: `periodSet` on a 16-bit timer would reject `u32::MAX`.
    #[test]
    fn unrepresentably_slow_frequencies_are_held() {
        // 1 Hz on a 120 MHz counter needs 120e6 counts, which fits; 0.01 Hz
        // needs 1.2e10, which does not.
        assert_eq!(pwm(120_000_000).period_counts_for(1.0), Some(120_000_000));
        assert_eq!(pwm(120_000_000).period_counts_for(0.01), None);
    }

    /// Nothing has been written to a freshly opened timer, so the first update
    /// must program the period and every duty even if the model happens to ask
    /// for what the configurator already set.
    #[test]
    fn a_fresh_wrapper_has_no_cached_period() {
        assert_eq!(pwm(120_000_000).period_counts, 0);
        assert!(pwm(120_000_000).period_counts_for(1_000.0) != Some(0));
    }

    #[test]
    fn duty_is_a_fraction_of_the_period() {
        let pwm = pwm(120_000_000);
        assert_eq!(pwm.duty_counts_for(1_000, 0.0), 0);
        assert_eq!(pwm.duty_counts_for(1_000, 0.25), 250);
        assert_eq!(pwm.duty_counts_for(1_000, 1.0), 1_000);
    }

    #[test]
    fn duty_clamps_to_zero_and_one() {
        let pwm = pwm(120_000_000);
        assert_eq!(pwm.duty_counts_for(1_000, -0.5), 0);
        assert_eq!(pwm.duty_counts_for(1_000, 1.5), 1_000);
        assert_eq!(pwm.duty_counts_for(1_000, f64::NAN), 0);
    }

    #[test]
    fn duty_counts_track_a_period_change() {
        // The reason `apply` re-applies every duty after `periodSet`: the same
        // fraction is a different count once the period moves, and periodSet
        // leaves the compare registers alone.
        let pwm = pwm(120_000_000);
        let slow = pwm.period_counts_for(1_000.0).unwrap();
        let fast = pwm.period_counts_for(2_000.0).unwrap();
        assert_eq!(pwm.duty_counts_for(slow, 0.5), 60_000);
        assert_eq!(pwm.duty_counts_for(fast, 0.5), 30_000);
    }
}
