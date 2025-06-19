use core::time::Duration;
use std::path::PathBuf;
use crate::loggers::Logger;

use super::{csv_logger::CsvLogger, udp_logger::UdpLogger};

/// LinuxLogger for Linux systems that logs data via UDP telemetry using
/// the device manager as well as a CSV file.
pub struct LinuxLogger {
    udp_logger: UdpLogger,
    csv_logger: CsvLogger,
}

impl LinuxLogger {
    pub fn new(
        udp_log_period: Duration,
        udp_socket: &str,
        csv_log_period: Duration,
        csv_output_path: PathBuf,
    ) -> Self {
        LinuxLogger {
            udp_logger: UdpLogger::new(udp_log_period, udp_socket),
            csv_logger: CsvLogger::new(csv_log_period, csv_output_path),
        }
    }
}

impl Logger for LinuxLogger {
    fn should_log(&mut self, app_time: Duration) -> bool {
        self.udp_logger.should_log(app_time) || self.csv_logger.should_log(app_time)
    }

    fn log(&mut self, log_data: &impl serde::Serialize, app_time: Duration) {
        self.udp_logger.log(log_data, app_time);
        self.csv_logger.log(log_data, app_time);
    }
}