#![no_std]
extern crate alloc;

mod clock_protocols;
pub use clock_protocols::*;

mod i2c_protocol;
pub use i2c_protocol::*;

mod gpio_protocol;
pub use gpio_protocol::*;
