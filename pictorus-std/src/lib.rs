//! This crate contains implementations of the various drivers needed to interact with I/O
//! on Linux-based platforms (i.e. Raspberry Pi). These are typically defined as `InputBlock`
//! or `OutputBlock` interfaces as defined in the `pictorus-traits` crate.
pub mod clock_protocol;
pub use clock_protocol::*;

pub mod delay_protocol;
pub use delay_protocol::*;

pub mod serial_protocol;
pub use serial_protocol::*;

pub mod udp_protocol;
pub use udp_protocol::*;
