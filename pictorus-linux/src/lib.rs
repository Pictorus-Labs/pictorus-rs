//! This crate contains implementations of the various drivers needed to interact with I/O
//! on Linux-based platforms (i.e. Raspberry Pi). These are typically defined as `InputBlock`
//! or `OutputBlock` interfaces as defined in the `pictorus-traits` crate.
mod clock_protocol;
pub use clock_protocol::*;

mod delay_protocol;
pub use delay_protocol::*;

mod gpio_protocol;
pub use gpio_protocol::*;

mod i2c_protocol;
pub use i2c_protocol::*;

mod pwm_protocol;
pub use pwm_protocol::*;

mod serial_protocol;
pub use serial_protocol::*;

mod udp_protocol;
pub use udp_protocol::*;

mod can_protocol;
pub use can_protocol::*;

mod spi_protocol;
pub use spi_protocol::*;
