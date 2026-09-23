//! Diagnostics for the hot path.
//!
//! A peripheral that fails once in a per-tick model usually fails every tick,
//! at whatever rate the model runs.

/// Log a warning the first time this call site is reached, and never again.
///
/// Each expansion gets its own flag, so one misbehaving peripheral does not
/// suppress the first report from another.
macro_rules! warn_once {
    ($($arg:tt)*) => {{
        static LOGGED: core::sync::atomic::AtomicBool =
            core::sync::atomic::AtomicBool::new(false);
        if !LOGGED.swap(true, core::sync::atomic::Ordering::Relaxed) {
            log::warn!($($arg)*);
        }
    }};
}

pub(crate) use warn_once;
