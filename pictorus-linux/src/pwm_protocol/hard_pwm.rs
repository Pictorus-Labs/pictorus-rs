use sysfs_pwm::Pwm as SysfsPwm;

const NANOSECONDS_PER_SECOND: f64 = 1_000_000_000.0;

pub struct HardPwm {
    pwm: SysfsPwm,
}

impl HardPwm {
    pub fn new(pin: f64) -> Result<Self, Box<dyn std::error::Error>> {
        // TODO: Allow selecting chip number
        let pwm = SysfsPwm::new(0, pin as u32)?;

        // Need to export/enable the pin before we can tell if it actually exists
        // Alternatively we could try to check npwm to see if the pin number is in range
        pwm.export()?;
        pwm.enable(true)?;
        Ok(Self { pwm })
    }
}

impl Drop for HardPwm {
    fn drop(&mut self) {
        self.pwm.unexport().ok();
    }
}

impl super::Pwm for HardPwm {
    type Channel = ();
    type Duty = f64;
    type Time = f64;

    fn disable(&mut self, _: Self::Channel) {
        self.pwm.enable(false).unwrap();
    }

    fn enable(&mut self, _: Self::Channel) {
        self.pwm.enable(true).unwrap();
    }

    fn get_duty(&self, _: Self::Channel) -> Self::Duty {
        self.pwm.get_duty_cycle_ns().unwrap() as f64 / self.pwm.get_period_ns().unwrap() as f64
    }

    fn get_max_duty(&self) -> Self::Duty {
        100.0
    }

    fn set_duty(&mut self, _: Self::Channel, duty: Self::Duty) {
        let duty_ns = (self.pwm.get_period_ns().unwrap() as f64 * duty).round() as u32;
        self.pwm.set_duty_cycle_ns(duty_ns).unwrap();
    }

    fn set_period<P>(&mut self, period: P)
    where
        P: Into<Self::Time>,
    {
        let period_ns = period.into() * NANOSECONDS_PER_SECOND;
        if period_ns == 0.0 {
            log::warn!("Period cannot be zero");
            return;
        }
        self.pwm.set_period_ns(period_ns as u32).unwrap();
    }

    fn get_period(&self) -> Self::Time {
        self.pwm.get_period_ns().unwrap() as f64 / NANOSECONDS_PER_SECOND
    }
}
