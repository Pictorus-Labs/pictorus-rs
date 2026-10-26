//! Runtime and peripheral adapters that back the Pictorus C FFI.
//!
//! The crate is linked into the per-app `ffi_lib` staticlib. Everything here is the same for
//! every app: the runtime (heap, panic handling, ABI version), the status codes, and the
//! adapters that let C-provided peripherals satisfy the `IoManager` contract of a generated
//! application. The per-app `app_interface_*` functions are generated, not defined here.
//!
//! The C-visible surface is described by `include/pictorus_ffi.h`, which is hand written.
//! Rust items that mirror it use the same names as the header, and tests check that the
//! constants agree.
#![no_std]
#![allow(non_camel_case_types)]

#[cfg(test)]
extern crate std;

pub mod logger;
pub mod peripherals;
pub mod runtime;
pub mod size;
pub mod status;

pub use logger::NoopLogger;
pub use runtime::{pictorus_rt_abi_version, pictorus_rt_config_t, pictorus_rt_init};
pub use status::*;
