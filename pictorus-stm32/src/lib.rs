//! This crate contains implementations of the various drivers needed to interact with I/O on STM32-based platforms.
//! These are typically defined as `InputBlock` or `OutputBlock` interfaces as defined in the `pictorus-traits` crate.
#![no_std]
extern crate alloc;

mod clock_protocol;
pub use clock_protocol::*;

mod serial_protocol;
pub use serial_protocol::*;

#[cfg(all(feature = "can", feature = "fdcan"))]
core::compile_error!("feature \"can\" and feature \"fdcan\" cannot be enabled at the same time");
#[cfg(any(feature = "can", feature = "fdcan"))]
mod can_protocol;
#[cfg(any(feature = "can", feature = "fdcan"))]
pub use can_protocol::*;

mod pwm_protocol;
pub use pwm_protocol::*;

mod i2c_protocol;
pub use i2c_protocol::*;

#[cfg(feature = "dac")]
mod dac_protocol;
#[cfg(feature = "dac")]
pub use dac_protocol::*;

#[cfg(feature = "spi")]
mod spi_protocol;
#[cfg(feature = "spi")]
pub use spi_protocol::*;

#[cfg(feature = "adc")]
mod adc_protocol;
#[cfg(feature = "adc")]
pub use adc_protocol::*;

mod gpio_protocol;
pub use gpio_protocol::*;
