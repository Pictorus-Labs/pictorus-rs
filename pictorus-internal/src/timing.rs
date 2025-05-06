use embedded_hal::delay::DelayNs;
use embedded_time::TimeInt;
use embedded_time::{Clock, Instant, duration::*};
use log::info;
use num_traits::AsPrimitive;

use crate::utils::s_to_us;

pub fn embedded_duration_to_us<T, U>(duration: Generic<T>) -> U
where
    T: TimeInt + AsPrimitive<U> + Copy,
    U: AsPrimitive<T> + Copy,
{
    let duration_us: Microseconds<T> = duration
        .try_into()
        .expect("Could not cast generic Duration to Microseconds!");
    duration_us.integer().as_()
}

#[derive(Debug)]
pub enum RunTime {
    Duration(u64), // Holds a finite duration in microseconds
    Indefinite,    // Special case for "run forever"
}

impl RunTime {
    pub fn from_f64_seconds(duration: f64) -> Self {
        if duration.is_finite() && duration >= 0.0 {
            RunTime::Duration(s_to_us(duration))
        } else {
            RunTime::Indefinite
        }
    }
}

pub struct Timing<C: Clock<T = u64>, D: DelayNs> {
    run_time: RunTime,
    iterations: u64,
    use_realtime: bool,
    timestep_us: u64,
    app_start_time: Instant<C>,
    loop_start_time: Instant<C>,
    clock: C,
    delay: D,
}

impl<C: Clock<T = u64>, D: DelayNs> Timing<C, D> {
    pub fn new(
        run_time: RunTime,
        hertz: f64,
        use_realtime: bool,
        clock: C,
        delay: D,
    ) -> Timing<C, D> {
        info!(
            "Timing settings: Run time: {:?}, frequency: {} hz, realtime: {}",
            run_time, hertz, use_realtime
        );
        let now = clock.try_now().unwrap();
        Timing {
            iterations: 0,
            use_realtime,
            run_time,
            timestep_us: s_to_us(1.0 / hertz),
            app_start_time: now,
            loop_start_time: now,
            clock,
            delay,
        }
    }

    pub fn update(&mut self, current_time_us: u64) -> u64 {
        self.maybe_sleep();

        self.loop_start_time = self.clock.try_now().unwrap();
        self.iterations += 1;

        // Return the updated app time
        self.update_app_time(current_time_us)
    }

    fn maybe_sleep(&mut self) {
        // Maybe put the app to sleep to maintain timing frequency.
        // Simulations (non-realtime) don't sleep.
        if !self.use_realtime {
            return;
        }

        let loop_duration_us: u64 =
            embedded_duration_to_us(self.clock.try_now().unwrap() - self.loop_start_time);
        if loop_duration_us >= self.timestep_us {
            return;
        }

        let remaining_time_us: u64 = self.timestep_us - loop_duration_us;
        self.delay.delay_us(remaining_time_us as u32);
    }

    pub fn should_run(&self, app_time_us: u64) -> bool {
        match self.run_time {
            RunTime::Indefinite => true,
            RunTime::Duration(duration) => app_time_us < duration,
        }
    }

    fn update_app_time(&self, current_time_us: u64) -> u64 {
        if !self.use_realtime {
            current_time_us + self.timestep_us
        } else {
            embedded_duration_to_us(self.clock.try_now().unwrap() - self.app_start_time)
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use embedded_time::Clock;
    use embedded_time::fraction::Fraction;

    // MockClock now takes a mutable reference to simulate advancing time in no_std.
    struct MockClock<'a> {
        time: &'a mut u64,
    }

    impl MockClock<'_> {
        fn advance(&mut self, duration: u64) {
            *self.time += duration;
        }
    }

    impl Clock for MockClock<'_> {
        type T = u64;

        const SCALING_FACTOR: Fraction = Fraction::new(1, 1);

        fn try_now(&self) -> Result<Instant<Self>, embedded_time::clock::Error> {
            Ok(Instant::new(*self.time))
        }
    }

    struct MockDelay;

    impl DelayNs for MockDelay {
        fn delay_ns(&mut self, _ns: u32) {
            // Mock delay does nothing
        }
    }

    fn init_timing(
        run_time: RunTime,
        hertz: f64,
        use_realtime: bool,
        time: &mut u64,
    ) -> Timing<MockClock<'_>, MockDelay> {
        let clock = MockClock { time };
        let delay = MockDelay;
        Timing::new(run_time, hertz, use_realtime, clock, delay)
    }

    #[test]
    fn test_timing_initialization() {
        let mut time = 0;
        let timing = init_timing(RunTime::Indefinite, 1.0, true, &mut time);
        assert_eq!(timing.iterations, 0);
        assert!(timing.use_realtime);
        assert_eq!(timing.timestep_us, 1_000_000); // 1 Hz = 1 second in microseconds
    }

    #[test]
    fn test_should_run_indefinite() {
        let mut time = 0;
        let timing = init_timing(RunTime::Indefinite, 1.0, true, &mut time);
        assert!(timing.should_run(0));
        assert!(timing.should_run(10_000_000)); // Arbitrary high value
    }

    #[test]
    fn test_should_run_duration() {
        let mut time = 0;
        let timing = init_timing(RunTime::Duration(5_000_000), 1.0, true, &mut time);
        assert!(timing.should_run(4_000_000)); // Less than 5 seconds
        assert!(!timing.should_run(6_000_000)); // More than 5 seconds
    }

    #[test]
    fn test_maybe_sleep_no_realtime() {
        let mut time = 0;
        let mut timing = init_timing(RunTime::Indefinite, 1.0, false, &mut time);
        let start_time = *timing.clock.time;
        timing.maybe_sleep();
        let end_time = *timing.clock.time;
        assert_eq!(start_time, end_time); // No sleep when use_realtime is false
    }

    #[test]
    fn test_maybe_sleep_realtime() {
        let mut time = 0;
        let mut timing = init_timing(RunTime::Indefinite, 1.0, true, &mut time);
        timing.clock.advance(500_000); // Simulate half the timestep has passed
        timing.maybe_sleep();
        // No actual delay happens since MockDelay does nothing, but logic is exercised.
    }

    #[test]
    fn test_update_increments_app_time() {
        let mut time = 0;
        let mut timing = init_timing(RunTime::Indefinite, 1.0, false, &mut time);
        let initial_app_time = timing.update(0);
        let updated_app_time = timing.update(initial_app_time);
        assert_eq!(updated_app_time, initial_app_time + timing.timestep_us);
    }
}
