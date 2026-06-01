#![no_std]
#[cfg(feature = "alloc")]
extern crate alloc;

mod clock_protocols;
pub use clock_protocols::*;

#[cfg(feature = "alloc")]
mod i2c_protocol;
#[cfg(feature = "alloc")]
pub use i2c_protocol::*;

mod gpio_protocol;
pub use gpio_protocol::*;
