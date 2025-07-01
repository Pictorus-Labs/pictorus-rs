use super::{Logger, PictorusLogger};
use core::time::Duration;
use miniserde::Serialize;
use rtt_target::rprintln;

const LOG_HEAP_MIN_PERIOD: Duration = Duration::from_secs(1);

/// RttLogger transmits data over the RTT protocol. Has an additional
/// method to log heap changes.
pub struct RttLogger {
    publish_period: Duration,
    last_broadcast_time: Option<Duration>,
    previous_heap_used: usize,
    last_heap_log_time: Duration,
}

impl RttLogger {
    pub fn new(publish_period: Duration) -> RttLogger {
        RttLogger {
            publish_period,
            last_broadcast_time: None,
            previous_heap_used: 0,
            last_heap_log_time: Duration::ZERO,
        }
    }

    /// Logs heap information if the heap size has changed since the last measurement.
    /// The heap doesn't live in the time series database, so it is logged separately
    /// as an [INFO] message.
    ///
    /// Currently logs only when a change in heap usage is detected.
    pub fn log_heap(&mut self, app_time: Duration, free: usize, used: usize) {
        // Only log heap usage if the heap usage has changed and at most once per second
        if self.previous_heap_used != used
            && app_time - self.last_heap_log_time >= LOG_HEAP_MIN_PERIOD
        {
            let free_f32 = free as f32 / 1000.0;
            let used_f32 = used as f32 / 1000.0;
            let percent_used = (used_f32 / (used_f32 + free_f32)) * 100.0;
            log::info!(
                "Heap Used: {used_f32:.3}kB, Heap Free: {free_f32:.3}kB, Heap Usage: {percent_used:.3}%",
            );
            self.previous_heap_used = used;
            self.last_heap_log_time = app_time;
        }
    }
}

impl PictorusLogger for RttLogger {
    fn add_samples(&mut self, log_data: &impl Serialize, app_time: Duration) {
        if self.should_log(app_time) {
            let sample = miniserde::json::to_string(log_data);
            self.log(app_time, &sample);
        }
    }
}

impl Logger for RttLogger {
    fn should_log(&mut self, app_time: Duration) -> bool {
        self.publish_period > Duration::ZERO
            && match self.last_broadcast_time {
                None => true, // Broadcast if there's no previous broadcast time
                Some(last_broadcast) => (app_time - last_broadcast) >= self.publish_period,
            }
    }

    fn log(&mut self, app_time: Duration, data: &str) {
        rprintln!("{}", data);
        self.last_broadcast_time = Some(app_time);
    }
}
