//! This crate contains all of the blocks available in the Pictorus UI.
//! These blocks are implemented using the traits defined in the `pictorus-traits`
//! crate.
//!
//! ## Block Types
//! There are currently three categories of blocks defined in this crate:
//! - Core Blocks - These are blocks that are available on all platforms (no `std`, no `alloc`).
//! - Alloc Blocks - These blocks require the `alloc` crate and are gated behind the `alloc` feature.
//!   The `alloc` feature is currently on by default; consumers in fully no-alloc environments
//!   should set `default-features = false`.
//! - Standard Blocks - These blocks require `std` and are gated behind the `std` feature.
//!
//! ## Implementing Custom Blocks
//! The blocks in this crate can be helpful as a starting point for implementing a custom block,
//! but are likely to be much more complex than required for a typical use case.
//! The blocks in this crate typically support a wide variety of input/output types,
//! and as such, need to use more complex patterns like macros and recursive traits to cover all possible cases.
//! If you are implementing a custom block for a specific case, you can likely create a much simpler implementation
//! by restricting the input/output types you need to support. We will be adding more examples of simple custom blocks in the future.
#![no_std]

#[cfg(any(feature = "std", test))]
extern crate std;

// Use alloc for tests. The `std` feature incorporates `alloc`
#[cfg(any(feature = "alloc", test))]
extern crate alloc;

// Set of blocks that do not depend on `std` or `alloc`
mod core_blocks;
pub use core_blocks::*;

// Set of blocks that depend on `alloc`
#[cfg(feature = "alloc")]
mod alloc_blocks;
#[cfg(feature = "alloc")]
pub use alloc_blocks::*;

// Set of blocks that depend on `std`
#[cfg(feature = "std")]
mod std_blocks;
#[cfg(feature = "std")]
pub use std_blocks::*;

#[cfg(feature = "alloc")]
pub mod byte_data;
mod matrix_ext;
pub use matrix_ext::{MatrixExt, MatrixNalgebraExt};
mod stale_tracker;
pub(crate) mod traits;
pub use traits::Scalar;

#[cfg(any(test, doctest))]
mod testing;

#[derive(Debug)]
/// Error raised when parsing an enum from a string fails.
pub struct ParseEnumError;
