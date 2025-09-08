//! # PX4 Messages System Bindings
//!
//! This crate provides low-level Rust bindings to PX4's uORB (micro Object Request Broker)
//! message system. It contains automatically generated Rust definitions for all PX4 message
//! types, allowing safe interoperation between Rust code and PX4's C/C++ flight control system.
//!
//! ## Usage
//!
//! This crate is typically not used directly, but rather through the higher-level
//! `pictorus-px4` crate which provides type-safe wrappers and integration with the
//! Pictorus block system.

#![no_std]

/// Generated PX4 message structure definitions
///
/// This module contains Rust struct definitions for all PX4 uORB message types.
/// These are generated using `bindgen` from PX4's C header files and have the same memory
/// layout as the original C structures.
#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(unused)]
pub mod message_defs;

/// uORB system definitions and metadata
///
/// This module contains the core uORB types and static metadata definitions.
/// Each message type has an associated `orb_metadata` struct that describes
/// its properties like size, name, and internal ID.
#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(unused)]
pub mod orb;
