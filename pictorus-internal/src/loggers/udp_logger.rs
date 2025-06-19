use core::time::Duration;
use log::{info, warn};
use std::{
    net::UdpSocket,
    string::{String, ToString},
};

use super::Logger;

/// The UdpLogger is used to transmit data over the UDP protocol to the device manager.
pub struct UdpLogger {
    pub file: Option<std::fs::File>,
    socket: Option<UdpSocket>,
    udp_publish_period: Duration,
    publish_socket: String,
    last_udp_publish_time: Option<Duration>,
    has_udp_connection: bool,
}

// Wait this long to re-establish connection to telemetry manager before giving up
const UDP_TIMEOUT: Duration = Duration::from_secs(10);

impl UdpLogger {
    pub fn new(publish_period: Duration, publish_socket: &str) -> Self {
        let socket = if publish_socket.is_empty() || publish_period.is_zero() {
            None
        } else {
            let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
            socket.set_nonblocking(true).unwrap();
            Some(socket)
        };

        UdpLogger {
            file: None,
            socket,
            udp_publish_period: publish_period,
            publish_socket: publish_socket.to_string(),
            last_udp_publish_time: None,
            has_udp_connection: true,
        }
    }
}

impl Logger for UdpLogger {
    fn should_log(&mut self, app_time: Duration) -> bool {
        self.udp_publish_period > Duration::ZERO
            && match self.last_udp_publish_time {
                None => true, // Broadcast if there's no previous broadcast time
                Some(last_broadcast) => (app_time - last_broadcast) >= self.udp_publish_period,
            }
    }

    fn log(&mut self, log_data: &impl serde::Serialize, app_time: Duration) {
        if self.should_log(app_time) {
            // TODO: Replace with Postcard + COBS, how to ensure the data is
            // separated by a sentinel bytes?
            let log_str = serde_json::to_string(log_data).unwrap();
            if let Some(socket) = &mut self.socket {
                let time_since_last_udp_publish = match self.last_udp_publish_time {
                    Some(last_publish_time) => app_time - last_publish_time,
                    None => app_time,
                };
                match socket.send_to(log_str.as_bytes(), &self.publish_socket) {
                    Ok(_) => {
                        self.last_udp_publish_time = Some(app_time);
                        if !self.has_udp_connection {
                            info!("Regained UDP connection.");
                            self.has_udp_connection = true;
                        }
                    }
                    Err(_) => {
                        if self.has_udp_connection {
                            warn!("Lost UDP connection! Skipping telemetry transmit...");
                            self.has_udp_connection = false;
                        } else if time_since_last_udp_publish > UDP_TIMEOUT {
                            panic!(
                                "Unable to connect to telemetry manager after {:?}, aborting.",
                                UDP_TIMEOUT
                            );
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct LogData {
        app_time: f64,
        current_state: String,
        foo_block: f64,
        bar_block: f64,
    }

    #[test]
    fn test_udp_data_logger_constructor() {
        let log_data = LogData {
            app_time: 1.0,
            current_state: "test_state".to_string(),
            foo_block: 0.0,
            bar_block: 1.0,
        };

        let logging_rate_hz: u64 = 10;
        let log_period = Duration::from_micros(1_000_000 / logging_rate_hz);
        let publish_socket = ""; // Dont publish for this test

        // Verify we can construct a DataLogger
        let mut dl = UdpLogger::new(log_period, publish_socket);
        let app_time = Duration::from_micros(1_234_000);
        // Verify we can pass it samples to log without errors
        dl.log(&log_data, app_time);
    }
}
