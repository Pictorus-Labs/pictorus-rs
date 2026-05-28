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
}
