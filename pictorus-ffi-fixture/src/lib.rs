//! A hand-written stand-in for a generated `ffi_lib`.
//!
//! It exposes the `app_interface_*` entry points described in `include/app_interface.h` over a
//! trivial model, so the runtime in `pictorus-ffi` can be exercised from C without needing the
//! code generator. As `pictorus-ffi` gains peripheral adapters the fixture grows to use them,
//! the way a generated crate would.
#![cfg_attr(target_os = "none", no_std)]

extern crate alloc;

use alloc::vec::Vec;
use core::mem::{align_of, size_of};
use core::sync::atomic::{AtomicUsize, Ordering};

use pictorus_ffi::{
    PICTORUS_ERR_INVALID_ARG, PICTORUS_OK, export_app_interface_size, pictorus_status_t,
};

const FUNDAMENTAL_TIMESTEP_US: u64 = 10_000;

static LIVE_INSTANCES: AtomicUsize = AtomicUsize::new(0);

#[repr(C)]
pub struct AppInputs {
    pub setpoint: f64,
}

#[repr(C)]
pub struct AppOutputs {
    pub output: f64,
    pub steps: u64,
}

/// Owns heap data so the tests cover the allocator, not only plain-old-data instances.
pub struct AppInterface {
    history: Vec<u64>,
}

export_app_interface_size!(AppInterface);

#[unsafe(no_mangle)]
pub extern "C" fn app_interface_size() -> usize {
    size_of::<AppInterface>()
}

#[unsafe(no_mangle)]
pub extern "C" fn app_interface_align() -> usize {
    align_of::<AppInterface>()
}

#[unsafe(no_mangle)]
pub extern "C" fn app_interface_fundamental_timestep_us() -> u64 {
    FUNDAMENTAL_TIMESTEP_US
}

/// # Safety
///
/// `mem` must be null, or valid for writes of `app_interface_size()` bytes, and must not be
/// used for anything else until `app_interface_deinit`. `out` must be null or valid for a write.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn app_interface_init(
    mem: *mut core::ffi::c_void,
    out: *mut *mut AppInterface,
) -> pictorus_status_t {
    if mem.is_null() || out.is_null() || !mem.cast::<AppInterface>().is_aligned() {
        return PICTORUS_ERR_INVALID_ARG;
    }

    let app = mem.cast::<AppInterface>();
    // SAFETY: `mem` is non-null, aligned, and large enough per this function's contract.
    unsafe {
        app.write(AppInterface {
            history: Vec::new(),
        });
        out.write(app);
    }
    LIVE_INSTANCES.fetch_add(1, Ordering::SeqCst);
    PICTORUS_OK
}

/// # Safety
///
/// `app` must come from `app_interface_init` and not yet be deinitialized. `inputs` and
/// `outputs` must be null or valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn app_interface_step(
    app: *mut AppInterface,
    app_time_us: u64,
    inputs: *const AppInputs,
    outputs: *mut AppOutputs,
) -> pictorus_status_t {
    // SAFETY: forwarded from this function's contract.
    let (Some(app), Some(inputs), Some(outputs)) = (
        unsafe { app.as_mut() },
        unsafe { inputs.as_ref() },
        unsafe { outputs.as_mut() },
    ) else {
        return PICTORUS_ERR_INVALID_ARG;
    };

    app.history.push(app_time_us);
    outputs.output = inputs.setpoint * 2.0;
    outputs.steps = app.history.len() as u64;
    PICTORUS_OK
}

/// # Safety
///
/// `app` must be null, or come from `app_interface_init` and not yet be deinitialized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn app_interface_deinit(app: *mut AppInterface) {
    if app.is_null() {
        return;
    }
    // SAFETY: forwarded from this function's contract.
    unsafe { app.drop_in_place() };
    LIVE_INSTANCES.fetch_sub(1, Ordering::SeqCst);
}

#[unsafe(no_mangle)]
pub extern "C" fn fixture_live_instances() -> usize {
    LIVE_INSTANCES.load(Ordering::SeqCst)
}
