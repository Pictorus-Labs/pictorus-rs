use block_data::BlockData;
use embedded_time::{duration::*, fraction::Fraction, Clock, Instant};

struct GenericClock;
impl Clock for GenericClock {
    type T = u64;

    const SCALING_FACTOR: Fraction = Fraction::new(1, 1_000_000);

    fn try_now(&self) -> Result<Instant<Self>, embedded_time::clock::Error> {
        panic!(
            "GenericClock is only used to calculate time elapsed. It cannot fetch the current time"
        )
    }
}

pub struct StaleTracker {
    stale_duration: Milliseconds<u64>,
    last_updated: Option<Instant<GenericClock>>,
}

// TODO: Update with core::time::Duration when all blocks are updated
impl StaleTracker {
    pub fn from_ms(age_ms: f64) -> Self {
        Self {
            stale_duration: Milliseconds(age_ms as u64),
            last_updated: None,
        }
    }

    fn seconds_to_instant(&self, app_time_s: f64) -> Instant<GenericClock> {
        let app_time_ms = (1000.0 * app_time_s) as u64;
        Instant::<GenericClock>::new(
            app_time_ms * GenericClock::SCALING_FACTOR.recip() * Fraction::new(1, 1000),
        )
    }

    // TODO: Update with core::time::Duration when all blocks are updated
    pub fn mark_updated(&mut self, app_time_s: f64) {
        let now = self.seconds_to_instant(app_time_s);
        self.last_updated = Some(now);
    }

    pub fn is_valid_bool(&self, app_time_s: f64) -> bool {
        match self.last_updated {
            None => false,
            Some(inst) => inst
                .checked_duration_until(&self.seconds_to_instant(app_time_s))
                .map(Milliseconds::<u64>::try_from)
                .and_then(Result::ok)
                .map(|dur| dur <= self.stale_duration)
                .unwrap_or(false),
        }
    }

    // TODO: Update with core::time::Duration when all blocks are updated
    pub fn is_valid(&self, app_time_s: f64) -> BlockData {
        BlockData::scalar_from_bool(self.is_valid_bool(app_time_s))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_is_valid_not_updated() {
        let tracker = StaleTracker::from_ms(5000.0);
        let valid = tracker.is_valid(0.0);
        assert!(!valid.all());
    }

    #[test]
    fn test_is_valid_updated_less_than_stale_duration() {
        let mut tracker = StaleTracker::from_ms(5000.0);
        tracker.mark_updated(0.0);
        let valid = tracker.is_valid(0.0);
        assert!(valid.all());
    }

    #[test]
    fn test_is_valid_updated_greater_than_stale_duration() {
        let mut tracker = StaleTracker::from_ms(1.0);
        tracker.mark_updated(0.0);
        let valid = tracker.is_valid(2.0);
        assert!(!valid.all());
    }
}
