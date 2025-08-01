use chrono::Utc;
use core::time::Duration;
use log::info;
use std::io::{BufWriter, Write};
use std::{fs::File, string::String};
use alloc::vec::Vec;

use super::Logger;

pub struct CsvLogger {
    last_csv_log_time: Option<Duration>,
    pub csv_log_period: Duration,
    pub writer: BufWriter<File>,
    pub output_path: std::path::PathBuf,
    pub app_start_epoch: Duration,
    header_written: bool,
    buffer: String,
}

impl CsvLogger {
    pub fn new(csv_log_period: Duration, output_path: std::path::PathBuf) -> Self {
        let file = if !csv_log_period.is_zero() {
            info!("DataLogger CSV output period: {csv_log_period:?}");
            info!("Streaming data output to file: {}", output_path.display());
            File::create(&output_path).unwrap()
        } else {
            info!("Not streaming output to file, logging rate set to zero.");
            File::create("/dev/null").unwrap()
        };

        CsvLogger {
            last_csv_log_time: None,
            csv_log_period,
            writer: BufWriter::with_capacity(65536, file), // 64KB buffer
            output_path,
            app_start_epoch: Duration::from_micros(
                Utc::now()
                    .timestamp_micros()
                    .try_into()
                    .expect("Could not cast app start epoch as u64"),
            ),
            header_written: false,
            buffer: String::with_capacity(1024), // Pre-allocate buffer
        }
    }

    fn write_csv_direct(&mut self, log_data: &impl serde::Serialize) {
        self.buffer.clear();
        
        // Get JSON representation just for structure, not for output
        let json = serde_json::to_value(log_data).unwrap();
        
        if let Some(json_map) = json.as_object() {
            if !self.header_written {
                // Write header
                let mut first = true;
                for (key, _) in json_map {
                    if !first {
                        self.buffer.push(',');
                    }
                    self.buffer.push_str(key);
                    first = false;
                }
                writeln!(self.writer, "{}", self.buffer).ok();
                self.header_written = true;
                self.buffer.clear();
            }
            
            // Write values
            let mut first = true;
            for (_, value) in json_map {
                if !first {
                    self.buffer.push(',');
                }
                
                match value {
                    serde_json::Value::Null => {},
                    serde_json::Value::Bool(b) => {
                        self.buffer.push_str(if *b { "true" } else { "false" });
                    },
                    serde_json::Value::Number(n) => {
                        use std::fmt::Write;
                        write!(self.buffer, "{}", n).ok();
                    },
                    serde_json::Value::String(s) => {
                        self.buffer.push('"');
                        self.buffer.push_str(s);
                        self.buffer.push('"');
                    },
                    serde_json::Value::Array(arr) => {
                        self.buffer.push('"');
                        self.buffer.push('[');
                        let mut first_elem = true;
                        for elem in arr {
                            if !first_elem {
                                self.buffer.push(',');
                            }
                            if let serde_json::Value::Number(n) = elem {
                                use std::fmt::Write;
                                write!(self.buffer, "{}", n).ok();
                            } else if let serde_json::Value::Array(inner) = elem {
                                // Handle nested arrays (like [[1,2,3]])
                                self.buffer.push('[');
                                let mut first_inner = true;
                                for inner_elem in inner {
                                    if !first_inner {
                                        self.buffer.push(',');
                                    }
                                    if let serde_json::Value::Number(n) = inner_elem {
                                        use std::fmt::Write;
                                        write!(self.buffer, "{}", n).ok();
                                    }
                                    first_inner = false;
                                }
                                self.buffer.push(']');
                            }
                            first_elem = false;
                        }
                        self.buffer.push(']');
                        self.buffer.push('"');
                    },
                    serde_json::Value::Object(_) => {
                        panic!("Unsupported data format for CSV samples");
                    }
                }
                first = false;
            }
            
            writeln!(self.writer, "{}", self.buffer).ok();
        }
    }
}

impl Logger for CsvLogger {
    fn should_log(&mut self, app_time: Duration) -> bool {
        self.csv_log_period > Duration::ZERO
            && match self.last_csv_log_time {
                None => true,
                Some(last_log) => (app_time - last_log) >= self.csv_log_period,
            }
    }

    fn log(&mut self, log_data: &impl serde::Serialize, app_time: Duration) {
        if self.should_log(app_time) {
            self.write_csv_direct(log_data);
            self.last_csv_log_time = Some(app_time);
        }
    }
}

impl Drop for CsvLogger {
    fn drop(&mut self) {
        // Ensure buffer is flushed on drop
        self.writer.flush().ok();
    }
}

pub fn format_header_csv(data: &impl serde::Serialize) -> String {
    let mut header = String::with_capacity(256);
    let json = serde_json::to_value(data).unwrap();
    let mut first_entry = true;
    if let Some(json_map) = json.as_object() {
        for (key, _) in json_map {
            if !first_entry {
                header.push(',');
            }
            header.push_str(key);
            first_entry = false;
        }
    }
    header
}

pub fn format_samples_csv(data: &impl serde::Serialize) -> String {
    let mut sample = String::with_capacity(512);
    let json = serde_json::to_value(data).unwrap();
    let mut first_entry = true;
    if let Some(json_map) = json.as_object() {
        for (_, value) in json_map {
            if !first_entry {
                sample.push(',');
            }

            match value {
                serde_json::Value::Null => {}
                serde_json::Value::Bool(b) => {
                    sample.push_str(if *b { "true" } else { "false" });
                }
                serde_json::Value::Number(n) => {
                    use std::fmt::Write;
                    write!(sample, "{}", n).ok();
                }
                serde_json::Value::String(s) => {
                    sample.push('"');
                    sample.push_str(s);
                    sample.push('"');
                }
                serde_json::Value::Array(values) => {
                    sample.push('"');
                    sample.push('[');
                    let mut first_elem = true;
                    for elem in values {
                        if !first_elem {
                            sample.push(',');
                        }
                        if let serde_json::Value::Number(n) = elem {
                            use std::fmt::Write;
                            write!(sample, "{}", n).ok();
                        } else if let serde_json::Value::Array(inner) = elem {
                            sample.push('[');
                            let mut first_inner = true;
                            for inner_elem in inner {
                                if !first_inner {
                                    sample.push(',');
                                }
                                if let serde_json::Value::Number(n) = inner_elem {
                                    use std::fmt::Write;
                                    write!(sample, "{}", n).ok();
                                }
                                first_inner = false;
                            }
                            sample.push(']');
                        } else {
                            use std::fmt::Write;
                            write!(sample, "{}", elem).ok();
                        }
                        first_elem = false;
                    }
                    sample.push(']');
                    sample.push('"');
                }
                serde_json::Value::Object(_) => {
                    panic!("Unsupported data format for CSV samples");
                }
            }
            first_entry = false;
        }
    }
    sample
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::string::ToString;
    use tempfile;

    #[derive(serde::Serialize)]
    struct TestLogData {
        state_id: Option<String>,
        timestamp: Option<f64>,
        utctime: Option<f64>,
        vector: Option<[[f64; 3]; 1]>,
        scalar: Option<f64>,
        matrix: Option<[[f64; 2]; 2]>,
        bytesarray: Option<[u8; 3]>,
    }

    // Original tests - still valid
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

        // last CSV write initialized to None
        assert_eq!(dl.last_csv_log_time, None);

        dl.log(&log_data, Duration::ZERO);
        assert_eq!(dl.last_csv_log_time, Some(Duration::ZERO));

        // Won't log again for 0.10s (10 hz)
        dl.log(&log_data, Duration::from_millis(1));
        assert_eq!(dl.last_csv_log_time, Some(Duration::ZERO));

        // This should update
        dl.log(&log_data, Duration::from_millis(123));
        assert_eq!(dl.last_csv_log_time, Some(Duration::from_millis(123)));
    }

    // New tests for the optimized implementation
    #[test]
    fn test_actual_file_output() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test.csv");
        
        let log_data = TestLogData {
            state_id: Some("test_state".to_string()),
            timestamp: Some(1.5),
            utctime: Some(2.5),
            vector: Some([[1.0, 2.0, 3.0]]),
            scalar: Some(42.0),
            matrix: None,
            bytesarray: None,
        };

        let mut logger = CsvLogger::new(Duration::from_micros(1), output_path.clone());
        
        // Log some data
        logger.log(&log_data, Duration::ZERO);
        
        // Force flush by dropping
        drop(logger);
        
        // Read file and verify contents
        let contents = std::fs::read_to_string(output_path).unwrap();
        assert!(contents.contains("state_id,timestamp,utctime,vector,scalar,matrix,bytesarray"));
        // Fix: Check for floats, not integers
        assert!(contents.contains("\"test_state\",1.5,2.5,\"[[1.0,2.0,3.0]]\",42"));
    }

    #[test]
    fn test_header_written_only_once() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test_header.csv");
        
        let log_data = TestLogData {
            state_id: Some("state1".to_string()),
            timestamp: Some(1.0),
            utctime: None,
            vector: None,
            scalar: Some(10.0),
            matrix: None,
            bytesarray: None,
        };

        let mut logger = CsvLogger::new(Duration::from_micros(1), output_path.clone());
        
        // Log multiple times
        logger.log(&log_data, Duration::ZERO);
        logger.log(&log_data, Duration::from_millis(100));
        logger.log(&log_data, Duration::from_millis(200));
        
        drop(logger);
        
        // Count header occurrences
        let contents = std::fs::read_to_string(output_path).unwrap();
        let header_count = contents.matches("state_id,timestamp").count();
        assert_eq!(header_count, 1, "Header should only appear once");
        
        // Should have 4 lines total (1 header + 3 data)
        let line_count = contents.lines().count();
        assert_eq!(line_count, 4);
    }

    #[test]
    fn test_buffer_reuse_no_data_leakage() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test_buffer.csv");
        
        let mut logger = CsvLogger::new(Duration::from_micros(1), output_path.clone());
        
        // First log with long string
        let log_data1 = TestLogData {
            state_id: Some("very_long_state_name_that_should_be_cleared".to_string()),
            timestamp: Some(1.0),
            utctime: None,
            vector: Some([[99.99, 88.88, 77.77]]),
            scalar: None,
            matrix: None,
            bytesarray: None,
        };
        
        // Second log with short string
        let log_data2 = TestLogData {
            state_id: Some("short".to_string()),
            timestamp: Some(2.0),
            utctime: None,
            vector: None,
            scalar: Some(1.0),
            matrix: None,
            bytesarray: None,
        };
        
        logger.log(&log_data1, Duration::ZERO);
        logger.log(&log_data2, Duration::from_millis(100));
        
        drop(logger);
        
        let contents = std::fs::read_to_string(output_path).unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        
        // Verify second line doesn't contain remnants of first
        assert!(!lines[2].contains("very_long"));
        assert!(!lines[2].contains("99.99"));
        assert!(lines[2].contains("\"short\""));
    }

    #[test]
    fn test_flush_on_drop() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test_flush.csv");
        
        let log_data = TestLogData {
            state_id: Some("flush_test".to_string()),
            timestamp: Some(123.456),
            utctime: None,
            vector: None,
            scalar: Some(789.0),
            matrix: None,
            bytesarray: None,
        };

        {
            let mut logger = CsvLogger::new(Duration::from_micros(1), output_path.clone());
            logger.log(&log_data, Duration::ZERO);
            // Logger dropped here - should flush
        }
        
        // Verify data was written
        let contents = std::fs::read_to_string(output_path).unwrap();
        assert!(contents.contains("flush_test"));
        assert!(contents.contains("123.456"));
        assert!(contents.contains("789"));
    }

    #[test]
    fn test_large_array_handling() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test_large.csv");
        
        // Create a struct with a large array
        #[derive(serde::Serialize)]
        struct LargeArrayData {
            #[serde(rename = "large_array")]
            large_array: Option<Vec<f64>>,
            id: Option<String>,
        }
        
        let large_data = LargeArrayData {
            large_array: Some((0..100).map(|i| i as f64 * 0.1).collect()),
            id: Some("large_test".to_string()),
        };
        
        let mut logger = CsvLogger::new(Duration::from_micros(1), output_path.clone());
        
        let start = std::time::Instant::now();
        logger.log(&large_data, Duration::ZERO);
        let elapsed = start.elapsed();
        
        drop(logger);
        
        // Should complete quickly even with large array
        assert!(elapsed < Duration::from_millis(10), "Large array logging took too long: {:?}", elapsed);
        
        // Verify data integrity
        let contents = std::fs::read_to_string(output_path).unwrap();
        assert!(contents.contains("large_test"));
        assert!(contents.contains("0,0.1,0.2")); // Check first few values
    }

    #[test]
    fn test_zero_logging_period() {
        // When period is zero, should not create real file
        let output_path = std::path::PathBuf::from("should_not_exist.csv");
        
        let log_data = TestLogData {
            state_id: Some("test".to_string()),
            timestamp: Some(1.0),
            utctime: None,
            vector: None,
            scalar: None,
            matrix: None,
            bytesarray: None,
        };
        
        let mut logger = CsvLogger::new(Duration::ZERO, output_path.clone());
        
        // Should not log anything
        logger.log(&log_data, Duration::ZERO);
        logger.log(&log_data, Duration::from_secs(1));
        
        drop(logger);
        
        // File should not exist
        assert!(!output_path.exists());
    }
}