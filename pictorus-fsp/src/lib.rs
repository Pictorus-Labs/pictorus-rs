//! Renesas FSP implementations of the Pictorus platform traits.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]

// Only for `From<FspError> for PictorusError`, which formats into two `String`s.
// Nothing in the protocol implementations allocates.
#[cfg(feature = "alloc")]
extern crate alloc;

mod diag;

mod error;

pub mod app;
pub mod gpio_protocol;
pub mod peripherals;
pub mod pwm_protocol;

pub use error::{FspError, Result};
pub use gpio_protocol::{FspInputPin, FspOutputPin, FspPin, Ioport};
pub use peripherals::Peripherals;
pub use pwm_protocol::FspPwm;
