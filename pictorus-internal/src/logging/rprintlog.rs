// This is a direct copy pasta of the rtt-logger crate.
// For some reason if I use the crate directly it breaks RTT comms.
// Think it might have something to do with a mismatch in the version of the rtt-target crate.
use log::{LevelFilter, Metadata, Record};

use rtt_target::rprintln;

/// An RTT-based logger implementation.
pub struct RPrintLog {
    level_filter: LevelFilter,
}

impl RPrintLog {
    /// Static-friendly const initializer.
    ///
    /// * `level_filter`: The default level to enable.
    pub const fn new(level_filter: LevelFilter) -> RPrintLog {
        RPrintLog { level_filter }
    }
}

impl log::Log for RPrintLog {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.level_filter.ge(&metadata.level())
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            rprintln!("[{}] - {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}
