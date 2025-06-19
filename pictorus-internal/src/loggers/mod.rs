use core::time::Duration;
use serde::Serialize;
#[cfg(feature = "std")]
pub mod csv_logger;

#[cfg(feature = "std")]
pub mod linux_logger;

#[cfg(feature = "std")]
pub mod udp_logger;

#[cfg(feature = "rtt")]
pub mod rtt_logger;

/// The Logger trait is used to log data to a file or transmit via telemetry.
///
/// Current implementations:
///
/// CsvLogger can be used to format and log CSV data to a file.
/// UdpLogger can be used to format and transmit telemetry data over UDP.
/// RttLogger can be used to transmit telemetry data over RTT.
pub trait Logger {
    /// Trait method to determine if the logger should log data based on the app's current elapsed
    /// time.
    fn should_log(&mut self, app_time: Duration) -> bool;

    /// Trait method to log data, with an option header parameter, for example, when first
    /// logging to a CSV file, a packet header, or comments. Calling this function should always
    /// result in data being logged. Use `should_log` to see if the logger should log data before
    /// calling this function.
    fn log(&mut self, log_data: &impl Serialize, app_time: Duration);
}
