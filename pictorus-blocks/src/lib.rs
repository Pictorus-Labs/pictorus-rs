//! All Pictorus blocks
#![no_std]
use block_data::BlockData;

#[cfg(any(feature = "std", test))]
extern crate std;

// TODO: Remoove this when we no longer require alloc
extern crate alloc;

// Set of blocks that do not depend on `std` or `alloc`
mod core_blocks;
pub use core_blocks::*;

// Set of blocks that depend on `std`
#[cfg(feature = "std")]
mod std_blocks;
#[cfg(feature = "std")]
pub use std_blocks::*;

pub mod byte_data;
mod nalgebra_interop;
mod stale_tracker;
pub(crate) mod traits;
pub use traits::Scalar;

#[cfg(any(test, doctest))]
mod testing;

#[derive(Debug)]
pub struct ParseEnumError;

pub trait IsValid {
    fn is_valid(&self, app_time_s: f64) -> BlockData;
}
