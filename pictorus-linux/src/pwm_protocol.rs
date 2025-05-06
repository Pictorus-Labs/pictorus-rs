use super::gpio_protocol::create_gpio_output_pin;
use embedded_hal_02::Pwm;
use pictorus_blocks::PwmBlockParams;
use pictorus_internal::protocols::{
    PWM_DUTY_CYCLE_TOLERANCE_12_BIT, PWM_PERIOD_TOLERANCE_POINT_1_US,
};
use pictorus_internal::utils::{PictorusError, positive_duration};
use pictorus_traits::OutputBlock;
use std::time::Duration;

mod soft_pwm;
use soft_pwm::SoftPwm;

mod hard_pwm;
use hard_pwm::HardPwm;

fn freq_to_period(frequency: f64) -> f64 {
    1.0 / frequency
}

fn duty_cycle_to_pulse_width(frequency: f64, duty_cycle: f64) -> f64 {
    freq_to_period(frequency) * duty_cycle
}

pub struct PwmConnection {
    hard_pwm: Option<HardPwm>,
    soft_pwm: Option<SoftPwm>,
    duty_cycle: f64,
    frequency: f64,
}

impl PwmConnection {
    pub fn new(pin_number: f64) -> Result<Self, PictorusError> {
        let hard_pwm = HardPwm::new(pin_number);
        let hard_pwm = match hard_pwm {
            Ok(pwm) => {
                log::debug!("Using hard PWM");
                Some(pwm)
            }
            Err(_) => None,
        };

        let frequency = 1.0;
        let duty_cycle = 0.0;

        let soft_pwm = match hard_pwm {
            Some(_) => None,
            None => {
                log::debug!("Using soft PWM");
                let pin = create_gpio_output_pin(pin_number)?;
                Some(SoftPwm::new(
                    pin,
                    positive_duration(freq_to_period(frequency)),
                    positive_duration(duty_cycle_to_pulse_width(frequency, duty_cycle)),
                ))
            }
        };

        Ok(Self {
            hard_pwm,
            soft_pwm,
            duty_cycle,
            frequency,
        })
    }

    fn period(&self) -> Duration {
        positive_duration(freq_to_period(self.frequency))
    }

    fn pulse_width(&self) -> Duration {
        positive_duration(duty_cycle_to_pulse_width(self.frequency, self.duty_cycle))
    }

    fn reconfigure_soft_pwm(&mut self) {
        let period_dur = self.period();
        let pulse_width_dur = self.pulse_width();
        if let Some(soft_pwm) = &mut self.soft_pwm {
            soft_pwm.reconfigure(period_dur, pulse_width_dur);
        }
    }
}

impl Pwm for PwmConnection {
    type Channel = ();
    type Duty = f64;
    type Time = f64;

    fn disable(&mut self, _: Self::Channel) {
        // This is kind of a dumb impl because you can't re-enable the PWM after stopping it.
        // In order to do that we need to figure out how to share ownership of the pin between
        // the SoftPwm thread and the PwmConnection struct (or pass ownership between them somehow?).
        if let Some(hard_pwm) = &mut self.hard_pwm {
            hard_pwm.disable(());
        }

        if let Some(soft_pwm) = &mut self.soft_pwm {
            soft_pwm.stop().ok();
        }
    }

    fn enable(&mut self, _: Self::Channel) {
        if let Some(hard_pwm) = &mut self.hard_pwm {
            hard_pwm.enable(());
        }
    }

    fn get_duty(&self, _: Self::Channel) -> Self::Duty {
        self.duty_cycle
    }

    fn get_max_duty(&self) -> Self::Duty {
        100.0
    }

    fn set_duty(&mut self, _: Self::Channel, duty: Self::Duty) {
        self.duty_cycle = duty;

        if let Some(hard_pwm) = &mut self.hard_pwm {
            hard_pwm.set_duty((), duty);
            return;
        }

        self.reconfigure_soft_pwm();
    }

    fn get_period(&self) -> Self::Time {
        self.period().as_secs_f64()
    }

    fn set_period<P>(&mut self, period: P)
    where
        P: Into<Self::Time>,
    {
        let period = period.into();
        self.frequency = 1.0 / period;

        if let Some(hard_pwm) = &mut self.hard_pwm {
            hard_pwm.set_period(period);
            return;
        }

        self.reconfigure_soft_pwm();
    }
}

pub fn create_pwm_protocol(pin_number: f64) -> Result<PwmConnection, PictorusError> {
    let conn = PwmConnection::new(pin_number)?;
    Ok(conn)
}

impl OutputBlock for PwmConnection {
    type Inputs = (f64, f64); // (Frequency, Duty Cycle)

    type Parameters = PwmBlockParams;

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) {
        let (frequency, duty_cycle) = inputs;

        let period = if frequency <= PWM_PERIOD_TOLERANCE_POINT_1_US {
            0.0
        } else {
            1.0 / frequency
        };

        if (self.get_period() - period).abs() >= PWM_PERIOD_TOLERANCE_POINT_1_US {
            self.set_period(period);
        }

        if (self.get_duty(()) - duty_cycle).abs() >= PWM_DUTY_CYCLE_TOLERANCE_12_BIT {
            self.set_duty((), duty_cycle);
        }
    }
}
