//! Bindings for `c/rm_pictorus_app.h`, plus the two entry points that belong to
//! this crate rather than to a generated application.
//!
//! # Division of labor
//!
//! The C shim owns the heap buffer, the vtable object and the tick loop. The
//! generated application crate owns the allocator, the panic handler, the
//! logger, `app_interface_{new,update,free}` and `pictorus_rt_heap_init`.
//! What is left is the part that is the same for every model: the ABI check
//! and the peripheral table.

use core::sync::atomic::{AtomicPtr, Ordering};

/// Raw bindings for `c/rm_pictorus_app.h`.
///
/// Public so that the generated application crate can define the entry points
/// the shim calls -- `app_interface_*`, `pictorus_rt_heap_init`,
/// `pictorus_rt_build_id` and the `g_pictorus_app_on_pictorus_app` vtable --
/// against the same declarations the shim was compiled from.
#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(clippy::all)]
pub mod seam {
    include!(concat!(env!("OUT_DIR"), "/seam.rs"));
}

pub use seam::{
    rm_pictorus_api_t, rm_pictorus_bindings_t, rm_pictorus_cfg_t, rm_pictorus_ctrl_t,
    rm_pictorus_instance_t,
};

/// Published by [`pictorus_rt_bind`], read by the model's I/O construction.
///
/// A pointer rather than a copy: everything it points at is a `const` global in
/// the generated `hal_data.c`, so it outlives the program and copying would
/// only duplicate it into RAM.
static BINDINGS: AtomicPtr<rm_pictorus_bindings_t> = AtomicPtr::new(core::ptr::null_mut());

/// Publish the peripheral table.
///
/// # Safety
///
/// `bindings`, and everything reachable from it, must remain valid for the rest
/// of the program. The generated `hal_data.c` satisfies this by emitting them
/// as `const` globals.
///
/// Must be called before any I/O wrapper is constructed, and exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pictorus_rt_bind(bindings: *const rm_pictorus_bindings_t) {
    BINDINGS.store(bindings.cast_mut(), Ordering::Release);
}

/// The peripheral table, or `None` before [`pictorus_rt_bind`] has run.
pub fn bindings() -> Option<&'static rm_pictorus_bindings_t> {
    // SAFETY: pictorus_rt_bind's contract makes the pointer valid for 'static.
    unsafe { BINDINGS.load(Ordering::Acquire).as_ref() }
}

/// Nothing in Rust calls `pictorus_rt_bind` -- C does -- so without a reference
/// the linker is entitled to discard it and the archive member it lives in.
/// Naming it from a `#[used]` static creates that reference.
///
/// Belt and braces: the pack's link line should also carry
/// `-Wl,--undefined=pictorus_rt_bind`, so that a regression here is a link
/// error rather than a module that silently binds nothing.
#[used]
static KEEP_EXPORTS: unsafe extern "C" fn(*const rm_pictorus_bindings_t) = pictorus_rt_bind;
