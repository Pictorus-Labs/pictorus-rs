//! This crate contains most of the dependencies referenced by generated code that are not part
//! of the core traits/blocks libraries. This include things like helper functions/structs for app execution,
//! logging, telemetry, etc. This crate is not intended to be used directly by users,
//! but rather as a dependency for the generated code.
#![no_std]
// TODO: We require alloc right now, but should be able to ditch this soon
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub mod execution_controller;
pub use execution_controller::ExecutionController;

pub mod runtime_context;
pub use runtime_context::RuntimeContext;

pub mod loggers;
pub mod protocols;
pub mod timing;
pub mod utils;
pub mod encoders;