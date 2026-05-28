use core::time::Duration;

#[derive(Default)]
pub struct StaleTracker {
    last_updated: Option<Duration>,
}

impl StaleTracker {
    pub fn mark_updated(&mut self, app_time: Duration) {
        self.last_updated = Some(app_time);
    }

    pub fn is_valid(&self, app_time: Duration, stale_duration: Duration) -> bool {
        self.last_updated
            .and_then(|inst| app_time.checked_sub(inst))
            .map(|elapsed| elapsed <= stale_duration)
            .unwrap_or(false)
    }
}

/// Convert a millisecond duration supplied as an `f64` into a `Duration`.
///
/// This will panic on negative, NaN, or infinite inputs.
pub fn duration_from_ms_f64(ms: f64) -> Duration {
    Duration::from_secs_f64(ms / 1000.0)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_is_valid_not_updated() {
        let tracker = StaleTracker::default();
        assert!(!tracker.is_valid(Duration::ZERO, Duration::from_secs(5)));
    }

    #[test]
    fn test_is_valid_updated_less_than_stale_duration() {
        let mut tracker = StaleTracker::default();
        tracker.mark_updated(Duration::ZERO);
        assert!(tracker.is_valid(Duration::ZERO, Duration::from_secs(5)));
    }

    #[test]
    fn test_is_valid_updated_greater_than_stale_duration() {
        let mut tracker = StaleTracker::default();
        tracker.mark_updated(Duration::ZERO);
        assert!(!tracker.is_valid(Duration::from_secs(2), Duration::from_millis(1)));
    }

    #[test]
    fn test_duration_from_ms_f64_normal() {
        assert_eq!(duration_from_ms_f64(1500.0), Duration::from_millis(1500));
        assert_eq!(duration_from_ms_f64(0.0), Duration::ZERO);
    }

    #[test]
    #[should_panic]
    fn test_duration_from_ms_f64_negative_input_panics() {
        duration_from_ms_f64(-1.0);
    }

    #[test]
    #[should_panic]
    fn test_duration_from_ms_f64_nan_input_panics() {
        duration_from_ms_f64(f64::NAN);
    }

    #[test]
    #[should_panic]
    fn test_duration_from_ms_f64_infinite_input_panics() {
        duration_from_ms_f64(f64::INFINITY);
    }

    #[test]
    #[should_panic]
    fn test_duration_from_ms_f64_neg_infinite_input_panics() {
        duration_from_ms_f64(f64::NEG_INFINITY);
    }
}
