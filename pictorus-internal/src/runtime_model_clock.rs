use core::time::Duration;
use pictorus_traits::ModelClock;

use crate::utils::us_to_s;

/// RuntimeModelClock is a small struct that implements the pictorus_traits::ModelClock trait.
/// It is used to keep track of time in the application and can be copied and cloned as
/// needed.
///
/// This is currently used in `context_module.py` to build out a codegen ModelClock that is
/// passed to each state.
#[derive(Clone, Copy)]
pub struct RuntimeModelClock {
    app_time_us: u64,
    fundamental_timestep_us: u64,
    last_app_time_us: Option<u64>,
}

impl RuntimeModelClock {
    pub fn new(fundamental_timestep_us: u64) -> Self {
        RuntimeModelClock {
            app_time_us: 0,
            fundamental_timestep_us,
            last_app_time_us: None,
        }
    }

    pub fn update_app_time(&mut self, app_time_us: u64) {
        self.last_app_time_us = Some(self.app_time_us);
        self.app_time_us = app_time_us;
    }

    pub fn app_time_s(&self) -> f64 {
        us_to_s(self.app_time_us)
    }

    pub fn app_time_us(&self) -> u64 {
        self.app_time_us
    }
}

impl ModelClock for RuntimeModelClock {
    fn fundamental_timestep(&self) -> Duration {
        Duration::from_micros(self.fundamental_timestep_us)
    }

    fn timestep(&self) -> Option<Duration> {
        self.last_app_time_us
            .map(|last_time| Duration::from_micros(self.app_time_us - last_time))
    }

    fn time(&self) -> Duration {
        Duration::from_micros(self.app_time_us)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_model_clock() {
        // Set timestep to 1000us or 1ms
        let mut model_clock = RuntimeModelClock::new(1000);
        assert_eq!(model_clock.fundamental_timestep(), Duration::from_micros(1000));

        model_clock.update_app_time(1000);
        assert_eq!(model_clock.time(), Duration::from_micros(1000));
        assert_eq!(model_clock.timestep().unwrap(), Duration::from_micros(1000));
        assert_eq!(model_clock.app_time_us(), 1000);
        assert_eq!(model_clock.app_time_s(), 0.001);
        assert_eq!(model_clock.fundamental_timestep(), Duration::from_micros(1000));

        model_clock.update_app_time(2000);
        assert_eq!(model_clock.time(), Duration::from_micros(2000));
        assert_eq!(model_clock.timestep().unwrap(), Duration::from_micros(1000));
        assert_eq!(model_clock.app_time_us(), 2000);
        assert_eq!(model_clock.app_time_s(), 0.002);
        assert_eq!(model_clock.fundamental_timestep(), Duration::from_micros(1000));

        // Covers the case where the timestep is not a multiple of the fundamental timestep - undershoot
        model_clock.update_app_time(2998);
        assert_eq!(model_clock.time(), Duration::from_micros(2998));
        assert_eq!(model_clock.timestep().unwrap(), Duration::from_micros(998));
        assert_eq!(model_clock.app_time_us(), 2998);
        assert_eq!(model_clock.app_time_s(), 0.002998);
        assert_eq!(model_clock.fundamental_timestep(), Duration::from_micros(1000));

        // Covers the case where the timestep is not a multiple of the fundamental timestep - overshoot
        model_clock.update_app_time(4010);
        assert_eq!(model_clock.time(), Duration::from_micros(4010));
        assert_eq!(model_clock.timestep().unwrap(), Duration::from_micros(1012));
        assert_eq!(model_clock.app_time_us(), 4010);
        assert_eq!(model_clock.app_time_s(), 0.00401);
        assert_eq!(model_clock.fundamental_timestep(), Duration::from_micros(1000));
    }
}
