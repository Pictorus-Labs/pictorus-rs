use core::time::Duration;
use pictorus_internal::loggers::Logger;
use serde::Serialize;

/// A [`Logger`] that never logs. The C library does not expose telemetry, but a generated app
/// that carries it still needs a logger to call into.
#[derive(Default, Clone, Copy)]
pub struct NoopLogger;

impl Logger for NoopLogger {
    fn should_log(&mut self, _app_time: Duration) -> bool {
        false
    }

    fn log(&mut self, _log_data: &impl Serialize, _app_time: Duration) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_logs() {
        let mut logger = NoopLogger;
        assert!(!logger.should_log(Duration::ZERO));
        assert!(!logger.should_log(Duration::from_secs(3600)));
    }
}
