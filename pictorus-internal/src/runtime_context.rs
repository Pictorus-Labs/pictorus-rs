use core::time::Duration;
use pictorus_traits::Context;

use crate::utils::us_to_s;

/// RuntimeContext is a small struct that implements the pictorus_traits::Context trait.
/// It is used to keep track of time in the application and can be copied and cloned as
/// needed.
///
/// This is currently used in `context_module.py` to build out a codegen Context that is
/// passed to each state.
#[derive(Clone, Copy)]
pub struct RuntimeContext {
    app_time_us: u64,
    fundamental_timestep_us: u64,
    last_app_time_us: Option<u64>,
}

impl RuntimeContext {
    pub fn new(fundamental_timestep_us: u64) -> Self {
        RuntimeContext {
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

impl Context for RuntimeContext {
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
    fn test_runtime_context() {
        // Set timestep to 1000us or 1ms
        let mut context = RuntimeContext::new(1000);
        assert_eq!(context.fundamental_timestep(), Duration::from_micros(1000));

        context.update_app_time(1000);
        assert_eq!(context.time(), Duration::from_micros(1000));
        assert_eq!(context.timestep().unwrap(), Duration::from_micros(1000));
        assert_eq!(context.app_time_us(), 1000);
        assert_eq!(context.app_time_s(), 0.001);
        assert_eq!(context.fundamental_timestep(), Duration::from_micros(1000));

        context.update_app_time(2000);
        assert_eq!(context.time(), Duration::from_micros(2000));
        assert_eq!(context.timestep().unwrap(), Duration::from_micros(1000));
        assert_eq!(context.app_time_us(), 2000);
        assert_eq!(context.app_time_s(), 0.002);
        assert_eq!(context.fundamental_timestep(), Duration::from_micros(1000));

        // Covers the case where the timestep is not a multiple of the fundamental timestep - undershoot
        context.update_app_time(2998);
        assert_eq!(context.time(), Duration::from_micros(2998));
        assert_eq!(context.timestep().unwrap(), Duration::from_micros(998));
        assert_eq!(context.app_time_us(), 2998);
        assert_eq!(context.app_time_s(), 0.002998);
        assert_eq!(context.fundamental_timestep(), Duration::from_micros(1000));

        // Covers the case where the timestep is not a multiple of the fundamental timestep - overshoot
        context.update_app_time(4010);
        assert_eq!(context.time(), Duration::from_micros(4010));
        assert_eq!(context.timestep().unwrap(), Duration::from_micros(1012));
        assert_eq!(context.app_time_us(), 4010);
        assert_eq!(context.app_time_s(), 0.00401);
        assert_eq!(context.fundamental_timestep(), Duration::from_micros(1000));
    }
}
