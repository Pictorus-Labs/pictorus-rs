//! This module is for things that implement the log::Log trait

#[cfg(feature = "rtt")]
mod rprintlog;

#[cfg(feature = "rtt")]
pub use rprintlog::RPrintLog;
