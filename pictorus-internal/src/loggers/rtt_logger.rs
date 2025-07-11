use rtt_target::UpChannel;

use super::Logger;
use crate::encoders::postcard_encoder::PostcardEncoderCOBS;
use core::time::Duration;

const LOG_HEAP_MIN_PERIOD: Duration = Duration::from_secs(1);

const RTT_DATA_BUFFER_SIZE: usize = 1024;

const RTT_LOG_SIZE: usize = 256;

pub fn pictorus_rtt_init() -> UpChannel {
    let channels = rtt_target::rtt_init! {
        up: {
            0: {
                size: RTT_DATA_BUFFER_SIZE,
                mode: rtt_target::ChannelMode::NoBlockSkip,
                name: "Data",
            }
            1: {
                size: RTT_LOG_SIZE,
                mode: rtt_target::ChannelMode::NoBlockSkip,
                name: "Log",
            }
        }
    };

    // Sets the print channel to the second up channel, rprint! (and log::debug, warn, etc)
    // will use this channel
    rtt_target::set_print_channel(channels.up.1);

    channels.up.0
}

/// RttLogger configures two RTT up channels named `Data` (1,024 bytes, NoBlockSkip) and
/// `Log` (256 bytes, NoBlockSkip). `Data` transmits u8 byte streams, while `Log` is
/// used for human readable messages using rprint! and rprintln! macros.
/// Has an additional method to log heap changes.
pub struct RttLogger {
    publish_period: Duration,
    last_broadcast_time: Option<Duration>,
    previous_heap_used: usize,
    last_heap_log_time: Duration,
    data_channel: UpChannel,
    encoder: PostcardEncoderCOBS,
}

impl RttLogger {
    pub fn new(publish_period: Duration) -> RttLogger {
        let data_channel = pictorus_rtt_init();
        RttLogger {
            publish_period,
            last_broadcast_time: None,
            previous_heap_used: 0,
            last_heap_log_time: Duration::ZERO,
            data_channel,
            encoder: PostcardEncoderCOBS {},
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

impl Logger for RttLogger {
    fn should_log(&mut self, app_time: Duration) -> bool {
        self.publish_period > Duration::ZERO
            && match self.last_broadcast_time {
                None => true, // Broadcast if there's no previous broadcast time
                Some(last_broadcast) => (app_time - last_broadcast) >= self.publish_period,
            }
    }

    fn log(&mut self, log_data: &impl serde::Serialize, app_time: Duration) {
        if self.should_log(app_time) {
            let encoded = self.encoder.encode::<RTT_DATA_BUFFER_SIZE>(log_data);
            self.data_channel.write(&encoded);
            self.last_broadcast_time = Some(app_time);
        }
    }
}
