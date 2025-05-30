//! This crate contains all of the blocks available in the Pictorus UI.
//! These blocks are implemented using the traits defined in the `pictorus-traits`
//! crate.
//!
//! ## Block Types
//! There are currently two categories of blocks defined in this crate:
//! - Core Blocks - These are blocks that are available on all platforms.
//! - Standard Blocks - These blocks are only available on platforms that support the standard library.
//!
//! ## Implementing Custom Blocks
//! The blocks in this crate can be helpful as a starting point for implementing a custom block,
//! but are likely to be much more complex than required for a typical use case.
//! The blocks in this crate typically support a wide variety of input/output types,
//! and as such, need to use more complex patterns like macros and recursive traits to cover all possible cases.
//! If you are implementing a custom block for a specific case, you can likely create a much simpler implementation
//! by restricting the input/output types you need to support. We will be adding more examples of simple custom blocks in the future.
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
/// Error raised when parsing an enum from a string fails.
pub struct ParseEnumError;

/// Trait for blocks that can have stale/invalid ouput.
///
/// Blocks that implement this trait should output a false value for `is_valid`
/// if the output is not in a valid state.
pub trait IsValid {
    // This still uses the deprecated BlockData type.
    // Eventually we will need to update/replace this trait when BlockData is removed.
    fn is_valid(&self, app_time_s: f64) -> BlockData;
}
