//! # Pictorus PX4 Integration Library
//!
//! This crate provides a bridge between the Pictorus block-based computation system and PX4's
//! flight control system. It enables PX4 modules to execute Pictorus computation graphs and
//! exchange data through PX4's uORB messaging system.
//!
//! ## Architecture Overview
//!
//! The integration works through a Foreign Function Interface (FFI) that allows:
//! - **C++ PX4 modules** to call into Rust computation code
//! - **Rust Pictorus blocks** to read from and write to uORB topics
//! - **Memory-safe data exchange** between the two systems
//!
//! ```text
//! ┌─────────────────┐    FFI     ┌──────────────────┐    uORB    ┌─────────────┐
//! │ PX4 C++ Module  │ ◄──────── │ Pictorus Rust    │ ◄───────── │ Flight      │
//! │                 │            │ Computation      │             │ Sensors     │
//! │ - Subscribes to │ ────────► │ - Input Blocks   │ ──────────► │ Actuators   │
//! │   uORB topics   │            │ - Output Blocks  │             │ Navigation  │
//! │ - Publishes     │            │ - FFI Protocol   │             │ etc.        │
//! │   results       │            │                  │             │             │
//! └─────────────────┘            └──────────────────┘             └─────────────┘
//! ```
//!
//! ## Core Components
//!
//! ### FFI Protocol ([`ffi_protocol`])
//!
//! The [`FfiProtocol`](ffi_protocol::FfiProtocol) manages bidirectional message passing:
//! - **Input Messages**: C++ writes sensor data, Rust reads for computation
//! - **Output Messages**: Rust writes computation results, C++ reads for publishing
//! - **Memory Management**: Safe handling of message buffers across language boundaries
//!
//! ### Message Implementations ([`message_impls`])
//!
//! Type-safe wrappers around PX4 message types providing:
//! - **Trait Implementations**: [`UorbMessage`](message_impls::UorbMessage), [`Topic`](message_impls::Topic) for all PX4 message types
//! - **Type Conversions**: Convert between PX4 structs and Pictorus data types
//! - **Memory Safety**: Proper alignment and lifetime management
//!
//! ### Pictorus Blocks
//!
//! - **[`FfiInputBlock<T>`](ffi_protocol::FfiInputBlock)**: Reads uORB messages into Pictorus computation graphs
//! - **[`FfiOutputBlock<T>`](ffi_protocol::FfiOutputBlock)**: Writes Pictorus results back to uORB system
//!
//! ## Usage Example
//!
//! ```rust
//! use pictorus_px4::ffi_protocol::{FfiProtocol, FfiInputBlock, FfiOutputBlock};
//! use pictorus_px4::message_impls::{SensorAccel, VehicleAttitudeSetpoint};
//!
//! // Set up the FFI protocol
//! let mut protocol = FfiProtocol::get_mut();
//! protocol.subscribe_to_message(SensorAccel::default());
//! protocol.advertise_message(VehicleAttitudeSetpoint::default());
//!
//! // Use in Pictorus blocks
//! let input_block = FfiInputBlock::<SensorAccel>::default();
//! let output_block = FfiOutputBlock::<VehicleAttitudeSetpoint>::default();
//! ```
//!
//! ## Safety Considerations
//!
//! This crate uses `unsafe` code for FFI and memory management. Key safety invariants:
//! - **FFI Functions**: All C-callable functions validate pointer arguments  
//! - **Memory Layout**: Message structs use `#[repr(C)]` for binary compatibility
//! - **Lifetime Management**: Static data structures ensure metadata remains valid
//! - **Critical Sections**: Single-threaded execution model simplifies synchronization
//!
//! ## PX4 Integration
//!
//! This library is designed to run within a PX4 module (see `module-shim/`):
//! 1. PX4 module calls `init_rust()` to initialize the system
//! 2. Module subscribes to required uORB topics  
//! 3. Each control loop iteration:
//!    - Module calls `rust_write_input_message()` for each subscribed topic
//!    - Module calls `step_rust()` to execute Pictorus computation
//!    - Module calls `rust_read_output_message()` for each advertised topic
//!    - Module publishes updated uORB messages
//!
//! ## No-std Compatibility
//!
//! This crate is `#![no_std]` compatible for embedded PX4 environments, using:
//! - Custom allocators ([`embedded-alloc`](https://docs.rs/embedded-alloc/))
//! - Lock-free data structures where possible
//! - Careful memory management with `alloc` collections

#![no_std]
extern crate alloc;

#[cfg(test)]
extern crate std;

/// FFI protocol for communication between PX4 C++ modules and Rust computation code
///
/// This module implements the core messaging protocol that allows PX4 modules to:
/// - Send sensor data to Rust for processing  
/// - Receive computation results back from Rust
/// - Manage message subscriptions and publications
///
/// See [`FfiProtocol`](ffi_protocol::FfiProtocol) for the main interface.
pub mod ffi_protocol;

/// Message type implementations and conversions for PX4 uORB messages
///
/// This module provides type-safe wrappers around all PX4 message types, implementing
/// traits for serialization, topic management, and integration with Pictorus data types.
///
/// Key traits:
/// - [`UorbMessage`](message_impls::UorbMessage): Core message serialization
/// - [`Topic`](message_impls::Topic): Topic metadata and identification  
/// - [`ToPassType`](message_impls::ToPassType)/[`FromPassType`](message_impls::FromPassType): Pictorus integration
pub mod message_impls;

/// Critical section implementation for single-threaded PX4 module context
pub struct CriticalSection;
critical_section::set_impl!(CriticalSection);

/// SAFETY: This implementation assumes single-threaded execution within a PX4 module.
/// In this context, critical sections are used to prevent preemption during
/// memory operations, but since we run in a single PX4 task with no interrupt
/// handlers modifying shared data, a NO-OP implementation is safe.
///
/// If this code were to be used in a multi-threaded context or with interrupt
/// handlers accessing shared data, proper critical section primitives would be required.
unsafe impl critical_section::Impl for CriticalSection {
    /// Acquire critical section - NO-OP for single-threaded PX4 module
    ///
    /// # Safety
    /// Safe in single-threaded context with no interrupt handlers modifying shared data
    unsafe fn acquire() -> critical_section::RawRestoreState {
        // NO-OP is safe in single-threaded PX4 module context
    }

    /// Release critical section - NO-OP for single-threaded PX4 module
    ///
    /// # Safety
    /// Safe in single-threaded context with no interrupt handlers modifying shared data
    unsafe fn release(_: critical_section::RawRestoreState) {
        // NO-OP is safe in single-threaded PX4 module context
    }
}
