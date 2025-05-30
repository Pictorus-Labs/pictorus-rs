use core::ops::Mul;
use embassy_stm32::Peripheral;
use embassy_stm32::gpio::OutputType;
use embassy_stm32::time::hz;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_stm32::timer::{
    self, Channel, Channel::Ch1, Channel1Pin, Channel2Pin, Channel3Pin, Channel4Pin,
};
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
        // save current duty cycle period is in seconds for use later
        let dc1 = self.get_duty_ch1();
        let dc2 = self.get_duty_ch2();
        let dc3 = self.get_duty_ch3();
        let dc4 = self.get_duty_ch4();
        // Disable to make changes to the frequency
        self.simple_pwm.disable(Channel::Ch1);
        self.simple_pwm.disable(Channel::Ch2);
        self.simple_pwm.disable(Channel::Ch3);
        self.simple_pwm.disable(Channel::Ch4);

        let freq = 1.0 / period.into();
        // Note: the hz function takes a u32 value and set_frequency asserts if freq == 0, the minimum
        // PWM frequency must be an integer of 1 or greater.
        self.simple_pwm.set_frequency(hz(freq as u32));

        // Embassy set frequency requires a duty cycle update, since the max duty cycle changes
        self.set_duty_ch1(dc1);
        self.set_duty_ch2(dc2);
        self.set_duty_ch3(dc3);
        self.set_duty_ch4(dc4);

        self.simple_pwm.enable(Channel::Ch1);
        self.simple_pwm.enable(Channel::Ch2);
        self.simple_pwm.enable(Channel::Ch3);
        self.simple_pwm.enable(Channel::Ch4);
    }
}

impl<T: timer::GeneralInstance4Channel> PwmWrapper<'_, T> {
    fn set_duty_ch1(&mut self, duty: f64) {
        if self.ch1.is_some() {
            self.set_duty(self.ch1.unwrap(), duty);
        }
    }

    fn set_duty_ch2(&mut self, duty: f64) {
        if self.ch2.is_some() {
            self.set_duty(self.ch2.unwrap(), duty);
        }
    }

    fn set_duty_ch3(&mut self, duty: f64) {
        if self.ch3.is_some() {
            self.set_duty(self.ch3.unwrap(), duty);
        }
    }

    fn set_duty_ch4(&mut self, duty: f64) {
        if self.ch4.is_some() {
            self.set_duty(self.ch4.unwrap(), duty);
        }
    }

    fn get_duty_ch1(&self) -> f64 {
        if self.ch1.is_some() {
            self.get_duty(self.ch1.unwrap())
        } else {
            0.0
        }
    }

    fn get_duty_ch2(&self) -> f64 {
        if self.ch2.is_some() {
            self.get_duty(self.ch2.unwrap())
        } else {
            0.0
        }
    }

    fn get_duty_ch3(&self) -> f64 {
        if self.ch3.is_some() {
            self.get_duty(self.ch3.unwrap())
        } else {
            0.0
        }
    }

    fn get_duty_ch4(&self) -> f64 {
        if self.ch4.is_some() {
            self.get_duty(self.ch4.unwrap())
        } else {
            0.0
        }
    }
}

impl<'d, T: timer::GeneralInstance4Channel> PwmWrapper<'d, T> {
    pub fn new(
        mut simple_pwm: SimplePwm<'d, T>,
        ch1: Option<Channel>,
        ch2: Option<Channel>,
        ch3: Option<Channel>,
        ch4: Option<Channel>,
    ) -> Self {
        simple_pwm.disable(Channel::Ch1);
        simple_pwm.disable(Channel::Ch2);
        simple_pwm.disable(Channel::Ch3);
        simple_pwm.disable(Channel::Ch4);
        simple_pwm.set_duty(Channel::Ch1, 0);
        simple_pwm.set_duty(Channel::Ch2, 0);
        simple_pwm.set_duty(Channel::Ch3, 0);
        simple_pwm.set_duty(Channel::Ch4, 0);

        PwmWrapper {
            simple_pwm,
            ch1,
            ch2,
            ch3,
            ch4,
        }
    }
}

impl<T: timer::GeneralInstance4Channel> OutputBlock for PwmWrapper<'_, T> {
    type Inputs = (f64, f64, f64, f64, f64); // (Frequency, Duty Cycle)

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

        if (self.get_duty_ch1() - duty_cycle1).abs() >= PWM_DUTY_CYCLE_TOLERANCE_16_BIT {
            self.set_duty_ch1(duty_cycle1);
        }

        if (self.get_duty_ch2() - duty_cycle2).abs() >= PWM_DUTY_CYCLE_TOLERANCE_16_BIT {
            self.set_duty_ch2(duty_cycle2);
        }

        if (self.get_duty_ch3() - duty_cycle3).abs() >= PWM_DUTY_CYCLE_TOLERANCE_16_BIT {
            self.set_duty_ch3(duty_cycle3);
        }

        if (self.get_duty_ch4() - duty_cycle4).abs() >= PWM_DUTY_CYCLE_TOLERANCE_16_BIT {
            self.set_duty_ch4(duty_cycle4);
        }
    }
}
