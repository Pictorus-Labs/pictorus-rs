use chrono::Utc;
use core::time::Duration;
use log::info;
use miniserde::ser::Fragment;
use std::io::Write;
use std::{fs::File, string::String};

use super::{Logger, PictorusLogger};

/// CsvLogger logs data to a file in CSV format.
///
/// Note, this uses a UTC time to be passed into the log. Other loggers
/// may use the app time in conjunction with the a device manager starting
/// timestamp to calculate the UTC time.
pub struct CsvLogger {
    last_csv_log_time: Option<Duration>,
    pub csv_log_period: Duration,
    pub file: std::fs::File,
    pub output_path: std::path::PathBuf,
    pub app_start_epoch: Duration,
}

impl CsvLogger {
    pub fn new(csv_log_period: Duration, output_path: std::path::PathBuf) -> Self {
        let mut file_obj = File::create("/dev/null").unwrap();
        if !csv_log_period.is_zero() {
            info!("DataLogger CSV output period: {csv_log_period:?}");
            info!("Streaming data output to file: {}", output_path.display());
            file_obj = File::create(std::path::PathBuf::from(&output_path)).unwrap();
        } else {
            info!("Not streaming output to file, logging rate set to zero.");
        }

        CsvLogger {
            last_csv_log_time: None,
            csv_log_period,
            file: file_obj,
            output_path,
            app_start_epoch: Duration::from_micros(
                Utc::now()
                    .timestamp_micros()
                    .try_into()
                    .expect("Could not cast app start epoch as u64"),
            ),
        }
    }
}

impl PictorusLogger for CsvLogger {
    fn add_samples(&mut self, log_data: &impl miniserde::Serialize, app_time: Duration) {
        if self.should_log(app_time) {
            let sample = format_samples_csv(log_data);
            if self.last_csv_log_time.is_none() {
                let header = format_header_csv(log_data);
                self.log(app_time, &header);
            }
            self.log(app_time, &sample);
        }
    }
}

impl Logger for CsvLogger {
    fn should_log(&mut self, app_time: Duration) -> bool {
        self.csv_log_period > Duration::ZERO
            && match self.last_csv_log_time {
                None => true, // Log if there's no previous log time
                Some(last_log) => (app_time - last_log) >= self.csv_log_period,
            }
    }

    fn log(&mut self, app_time: Duration, data: &str) {
        writeln!(self.file, "{data}").ok();
        self.last_csv_log_time = Some(app_time);
    }
}

/// Formats the header for CSV output based on the provided data.
/// This function extracts the field names from the data and formats them as a CSV header.
pub fn format_header_csv(data: &impl miniserde::Serialize) -> String {
    let mut header = String::new();
    match data.begin() {
        Fragment::Map(mut fields) => {
            // We are manually updating `field` with calls to `fields.next()` so
            // that we can avoid adding a trailing comma to the last field.
            let mut field = fields.next();
            while let Some(ref field_inner) = field {
                header.push_str(&field_inner.0);
                field = fields.next();
                if field.is_some() {
                    // Add a comma between fields
                    header.push(',');
                }
            }
        }
        _ => {
            panic!("Unsupported data format for CSV header");
        }
    }
    header
}

/// Formats the samples for CSV output based on the provided data.
pub fn format_samples_csv(data: &impl miniserde::Serialize) -> String {
    let mut sample = String::new();
    match data.begin() {
        Fragment::Map(mut fields) => {
            // We do the same trick as in `format_header_csv` to avoid a trailing comma.
            let mut field = fields.next();
            while let Some(ref field_inner) = field {
                match field_inner.1.begin() {
                    Fragment::Null => {} // This indicates a None value, we just have it be empty
                    Fragment::Seq(_) => {
                        // This indicates an array, need to add `"` to avoid parsers splitting the array up incorrectly
                        sample.push('"');
                        sample.push_str(&miniserde::json::to_string(field_inner.1));
                        sample.push('"');
                    }
                    Fragment::Map(_) => {
                        // We don't support this or expect to see it
                        panic!("Unsupported data format for CSV samples");
                    }
                    Fragment::Bool(_)
                    | Fragment::F64(_)
                    | Fragment::I64(_)
                    | Fragment::U64(_)
                    | Fragment::Str(_) => {
                        // Simple scalar values don't need any special formatting
                        sample.push_str(&miniserde::json::to_string(field_inner.1));
                    }
                }
                field = fields.next();
                if field.is_some() {
                    // Add a comma between fields
                    sample.push(',');
                }
            }
        }
        _ => {
            panic!("Unsupported data format for CSV samples");
        }
    }
    sample
}

#[cfg(test)]
mod tests {
    use std::string::ToString;

    use super::*;
    use crate::loggers::PictorusLogger;

    #[derive(miniserde::Serialize)]
    struct TestLogData {
        state_id: Option<String>,
        timestamp: Option<f64>,
        utctime: Option<f64>,
        vector: Option<[[f64; 3]; 1]>,
        scalar: Option<f64>,
        matrix: Option<[[f64; 2]; 2]>,
        bytesarray: Option<[u8; 3]>,
    }

    #[test]
    fn test_csv_formatting() {
        let log_data = TestLogData {
            state_id: "main_state".to_string().into(),
            timestamp: 1.234.into(),
            utctime: 2.234.into(),
            vector: Some([[0.0, 2.0, 4.0]]),
            scalar: 1.0.into(),
            matrix: Some([[5.0, 6.0], [7.0, 8.0]]),
            bytesarray: Some([1, 2, 3]),
        };

        let csv_header = format_header_csv(&log_data);
        assert_eq!(
            csv_header,
            "state_id,timestamp,utctime,vector,scalar,matrix,bytesarray".to_string()
        );

        let csv_data = format_samples_csv(&log_data);
        assert_eq!(
            csv_data,
            ("\"main_state\",1.234,2.234,\"[[0.0,2.0,4.0]]\",1.0,\"[[5.0,6.0],[7.0,8.0]]\",\"[1,2,3]\"")
        );
    }

    #[test]
    fn test_csv_formatting_empty_fields() {
        // // Verify we can format samples of different array types for CSV logging without errors
        let log_data = TestLogData {
            state_id: Some("main_state".to_string()),
            timestamp: Some(1.234),
            utctime: Some(2.234),
            vector: None,
            scalar: None,
            matrix: None,
            bytesarray: None,
        };

        let csv_header = format_header_csv(&log_data);
        assert_eq!(
            csv_header,
            "state_id,timestamp,utctime,vector,scalar,matrix,bytesarray".to_string()
        );
        let csv_data = format_samples_csv(&log_data);
        assert_eq!(csv_data, ("\"main_state\",1.234,2.234,,,,"));
    }

    #[test]
    fn test_data_logger_csv_update() {
        let log_data = TestLogData {
            state_id: "main_state".to_string().into(),
            timestamp: 1.234.into(),
            utctime: 2.234.into(),
            vector: Some([[0.0, 2.0, 4.0]]),
            scalar: 1.0.into(),
            matrix: Some([[5.0, 6.0], [7.0, 8.0]]),
            bytesarray: Some([1, 2, 3]),
        };

        let logging_rate_hz: u64 = 10; // 10 hz
        let log_period = Duration::from_micros(1_000_000 / logging_rate_hz);
        let output_path = std::path::PathBuf::from("/dev/null");

        let mut dl = CsvLogger::new(log_period, output_path);

        // last CSV write initialized to u64::MAX
        assert_eq!(dl.last_csv_log_time, None);

        dl.add_samples(&log_data, Duration::ZERO);
        assert_eq!(dl.last_csv_log_time, Some(Duration::ZERO));

        // Won't log again for 0.10s (10 hz)
        dl.add_samples(&log_data, Duration::from_millis(1));
        assert_eq!(dl.last_csv_log_time, Some(Duration::ZERO));

        // This should update
        dl.add_samples(&log_data, Duration::from_millis(123));
        assert_eq!(dl.last_csv_log_time, Some(Duration::from_millis(123)));
    }
}
