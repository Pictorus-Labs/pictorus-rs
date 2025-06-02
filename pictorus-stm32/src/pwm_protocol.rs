use core::ops::Mul;
use embassy_stm32::time::hz;
use embassy_stm32::timer::simple_pwm::SimplePwm;
use embassy_stm32::timer::{self, Channel};
use embedded_hal_02::Pwm;
use pictorus_blocks::PwmBlockParams;
use pictorus_internal::protocols::{
    PWM_DUTY_CYCLE_TOLERANCE_16_BIT, PWM_PERIOD_TOLERANCE_POINT_1_US,
};
use pictorus_traits::OutputBlock;

pub struct PwmWrapper<'d, T: timer::GeneralInstance4Channel> {
    simple_pwm: SimplePwm<'d, T>,
    ch1: Option<Channel>,
    ch2: Option<Channel>,
    ch3: Option<Channel>,
    ch4: Option<Channel>,
}

impl<T: timer::GeneralInstance4Channel> Pwm for PwmWrapper<'_, T> {
    type Channel = Channel;

    type Time = f64;

    type Duty = f64;

    fn disable(&mut self, channel: Self::Channel) {
        self.simple_pwm.disable(channel)
    }

    fn enable(&mut self, channel: Self::Channel) {
        self.simple_pwm.enable(channel)
    }

    fn get_period(&self) -> Self::Time {
        // This seems to return the frequency, not the period, so we need to invert it
        let p = self.simple_pwm.get_period().0 as f64;
        1.0 / p
    }

    /// Gets the duty cycle from 0 to 1
    fn get_duty(&self, channel: Self::Channel) -> Self::Duty {
        let max_dc = self.simple_pwm.get_max_duty() as f64;
        let dc = self.simple_pwm.get_duty(channel) as f64;
        dc / max_dc
    }

    // Gets the max duty cycle in timer ticks
    fn get_max_duty(&self) -> Self::Duty {
        self.simple_pwm.get_max_duty() as f64
    }

    /// Sets the duty cycle from 0 to 1
    fn set_duty(&mut self, channel: Self::Channel, duty: Self::Duty) {
        let max_duty = self.simple_pwm.get_max_duty();
        let clamped_dc = duty.clamp(0.0, 1.0) as f32;
        let duty_final_u32 = clamped_dc.mul(max_duty as f32) as u32;
        self.simple_pwm.set_duty(channel, duty_final_u32);
    }

    fn set_period<P>(&mut self, period: P)
    where
        P: Into<Self::Time>,
    {
        // save current duty cycle to use after frequency change
        let (dc1, dc2, dc3, dc4) = self.get_duty_cycle_all();

        // Disable to make changes to the frequency
        self.disable_all();

        let freq = 1.0 / period.into();
        // Note: the hz function takes a u32 value and set_frequency asserts if freq == 0, the minimum
        // PWM frequency must be an integer of 1 or greater.
        self.simple_pwm.set_frequency(hz(freq as u32));

        // Embassy set frequency requires a duty cycle update, since the max duty cycle changes
        self.set_duty_cycle_all((dc1, dc2, dc3, dc4));

        self.enable_all();
    }
}

impl<T: timer::GeneralInstance4Channel> PwmWrapper<'_, T> {
    fn disable_all(&mut self) {
        self.disable_channel(self.ch1);
        self.disable_channel(self.ch2);
        self.disable_channel(self.ch3);
        self.disable_channel(self.ch4);
    }

    fn enable_all(&mut self) {
        self.enable_channel(self.ch1);
        self.enable_channel(self.ch2);
        self.enable_channel(self.ch3);
        self.enable_channel(self.ch4);
    }

    fn get_duty_cycle_all(&self) -> (f64, f64, f64, f64) {
        (
            self.get_duty_cycle(self.ch1),
            self.get_duty_cycle(self.ch2),
            self.get_duty_cycle(self.ch3),
            self.get_duty_cycle(self.ch4),
        )
    }

    fn set_duty_cycle_all(&mut self, duty_cycle: (f64, f64, f64, f64)) {
        self.set_duty_cycle(self.ch1, duty_cycle.0);
        self.set_duty_cycle(self.ch2, duty_cycle.1);
        self.set_duty_cycle(self.ch3, duty_cycle.2);
        self.set_duty_cycle(self.ch4, duty_cycle.3);
    }

    fn enable_channel(&mut self, channel: Option<Channel>) {
        if let Some(ch) = channel {
            self.simple_pwm.enable(ch);
        }
    }

    fn disable_channel(&mut self, channel: Option<Channel>) {
        if let Some(ch) = channel {
            self.simple_pwm.disable(ch);
        }
    }

    fn set_duty_cycle(&mut self, channel: Option<Channel>, duty: f64) {
        if let Some(ch) = channel {
            self.set_duty(ch, duty);
        }
    }

    fn get_duty_cycle(&self, channel: Option<Channel>) -> f64 {
        if let Some(ch) = channel {
            self.get_duty(ch)
        } else {
            0.0
        }
    }

    fn maybe_update_duty_cycle(&mut self, channel: Option<Channel>, duty: f64) {
        if (self.get_duty_cycle(channel) - duty).abs() >= PWM_DUTY_CYCLE_TOLERANCE_16_BIT {
            self.set_duty_cycle(channel, duty);
        }
    }
}

impl<'d, T: timer::GeneralInstance4Channel> PwmWrapper<'d, T> {
    pub fn new(
        simple_pwm: SimplePwm<'d, T>,
        ch1: Option<Channel>,
        ch2: Option<Channel>,
        ch3: Option<Channel>,
        ch4: Option<Channel>,
    ) -> Self {
        let mut wrapper = PwmWrapper {
            simple_pwm,
            ch1,
            ch2,
            ch3,
            ch4,
        };

        wrapper.disable_all(); // Disable all channels initially
        wrapper.set_duty_cycle_all((0.0, 0.0, 0.0, 0.0)); // Set initial duty cycles to 0

        wrapper
    }
}

impl<T: timer::GeneralInstance4Channel> OutputBlock for PwmWrapper<'_, T> {
    type Inputs = (f64, f64, f64, f64, f64); // (Frequency, Duty Cycle Ch1, Duty Cycle Ch2, Duty Cycle Ch3, Duty Cycle Ch4)

    type Parameters = PwmBlockParams;

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) {
        let (frequency, duty_cycle1, duty_cycle2, duty_cycle3, duty_cycle4) = inputs;

        let period = f64::min(1.0, 1.0 / frequency);

        if (self.get_period() - period).abs() >= PWM_PERIOD_TOLERANCE_POINT_1_US {
            self.set_period(period);
        }

        self.maybe_update_duty_cycle(self.ch1, duty_cycle1);
        self.maybe_update_duty_cycle(self.ch2, duty_cycle2);
        self.maybe_update_duty_cycle(self.ch3, duty_cycle3);
        self.maybe_update_duty_cycle(self.ch4, duty_cycle4);
    }
}
