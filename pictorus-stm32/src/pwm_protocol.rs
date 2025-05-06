use core::ops::Mul;
use embassy_stm32::Peripheral;
use embassy_stm32::gpio::OutputType;
use embassy_stm32::time::hz;
use embassy_stm32::timer::simple_pwm::PwmPin;
use embassy_stm32::timer::simple_pwm::SimplePwm;
use embassy_stm32::timer::{self, Channel, Channel1Pin};
use embedded_hal_02::Pwm;
use pictorus_blocks::PwmBlockParams;
use pictorus_internal::protocols::{
    PWM_DUTY_CYCLE_TOLERANCE_16_BIT, PWM_PERIOD_TOLERANCE_POINT_1_US,
};
use pictorus_traits::OutputBlock;

pub struct PwmWrapper<'d, T: timer::GeneralInstance4Channel> {
    simple_pwm: SimplePwm<'d, T>,
}

impl<T: timer::GeneralInstance4Channel> Pwm for PwmWrapper<'_, T> {
    // The Pi only supports 1 PWM channel per PWM timer module. The STM32
    // supports 4 PWM channels per PWM timer for several timers, but to be
    // consistent with the Pi, only one channel, CH1, will be used with
    // each timer.
    type Channel = ();

    type Time = f64;

    type Duty = f64;

    fn disable(&mut self, _channel: Self::Channel) {
        self.simple_pwm.disable(Channel::Ch1);
    }

    fn enable(&mut self, _channel: Self::Channel) {
        self.simple_pwm.enable(Channel::Ch1)
    }

    fn get_period(&self) -> Self::Time {
        // This seems to return the frequency, not the period, so we need to invert it
        let p = self.simple_pwm.get_period().0 as f64;
        1.0 / p
    }

    /// Gets the duty cycle from 0 to 1
    fn get_duty(&self, _channel: Self::Channel) -> Self::Duty {
        let max_dc = self.simple_pwm.get_max_duty() as f64;
        let dc = self.simple_pwm.get_duty(Channel::Ch1) as f64;
        dc / max_dc
    }

    // Gets the max duty cycle in timer ticks
    fn get_max_duty(&self) -> Self::Duty {
        self.simple_pwm.get_max_duty() as f64
    }

    /// Sets the duty cycle from 0 to 1
    fn set_duty(&mut self, _channel: Self::Channel, duty: Self::Duty) {
        let max_duty = self.simple_pwm.get_max_duty();
        let clamped_dc = duty.clamp(0.0, 1.0) as f32;
        let duty_final_u32 = clamped_dc.mul(max_duty as f32) as u32;
        self.simple_pwm.set_duty(Channel::Ch1, duty_final_u32);
    }

    fn set_period<P>(&mut self, period: P)
    where
        P: Into<Self::Time>,
    {
        // save current duty cycle period is in seconds for use later
        let dc = self.get_duty(());
        // Disable to make changes to the frequency
        self.simple_pwm.disable(Channel::Ch1);
        let freq = 1.0 / period.into();
        // Note: the hz function takes a u32 value and set_frequency asserts if freq == 0, the minimum
        // PWM frequency must be an integer of 1 or greater.
        self.simple_pwm.set_frequency(hz(freq as u32));
        // Embassy set frequency requires a duty cycle update, since the max duty cycle changes
        self.set_duty((), dc);
        self.simple_pwm.enable(Channel::Ch1);
    }
}

impl<'d, T: timer::GeneralInstance4Channel> PwmWrapper<'d, T> {
    pub fn new(
        timer1: impl Peripheral<P = T> + 'd,
        pin_channel_1: Option<impl Peripheral<P = impl Channel1Pin<T>> + 'd>,
    ) -> Self {
        let ch1_pin = match pin_channel_1.is_some() {
            true => Some(PwmPin::new_ch1(
                pin_channel_1.expect("Valid TIMER = T pin 1"),
                OutputType::PushPull,
            )),
            false => None,
        };

        let mut simple_pwm: SimplePwm<'_, T> =
            SimplePwm::new(timer1, ch1_pin, None, None, None, hz(1), Default::default());

        simple_pwm.disable(Channel::Ch1);

        simple_pwm.set_duty(Channel::Ch1, 0);

        PwmWrapper { simple_pwm }
    }
}

impl<T: timer::GeneralInstance4Channel> OutputBlock for PwmWrapper<'_, T> {
    type Inputs = (f64, f64); // (Frequency, Duty Cycle)

    type Parameters = PwmBlockParams;

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) {
        let (frequency, duty_cycle) = inputs;

        let period = f64::min(1.0, 1.0 / frequency);

        if (self.get_period() - period).abs() >= PWM_PERIOD_TOLERANCE_POINT_1_US {
            self.set_period(period);
        }

        if (self.get_duty(()) - duty_cycle).abs() >= PWM_DUTY_CYCLE_TOLERANCE_16_BIT {
            self.set_duty((), duty_cycle);
        }
    }
}
