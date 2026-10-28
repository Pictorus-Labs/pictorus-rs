//! Exports the size and alignment of an app instance as link-time symbols.
//!
//! An integrator that carves instance storage out of a linker script can't call
//! `app_interface_size()`, and C can't size a static array from a linker symbol. So the build
//! exports `PICTORUS_APP_INTERFACE_SIZE` and `PICTORUS_APP_INTERFACE_ALIGN` as absolute symbols
//! holding the values for the exact target being built, which the export pipeline can then read
//! out of the finished archive (for example with `nm -S`) to fill in the generated header.
//!
//! The values can only be known by the crate that defines the instance type, hence a macro for
//! it to invoke rather than something this crate can do itself.

/// Defines the `PICTORUS_APP_INTERFACE_SIZE` and `PICTORUS_APP_INTERFACE_ALIGN` symbols for
/// `$t`. Invoke once, in the crate that defines the app instance type.
#[cfg(not(target_vendor = "apple"))]
#[macro_export]
macro_rules! export_app_interface_size {
    ($t:ty) => {
        core::arch::global_asm!(
            ".globl PICTORUS_APP_INTERFACE_SIZE",
            ".set PICTORUS_APP_INTERFACE_SIZE, {size}",
            ".globl PICTORUS_APP_INTERFACE_ALIGN",
            ".set PICTORUS_APP_INTERFACE_ALIGN, {align}",
            size = const core::mem::size_of::<$t>(),
            align = const core::mem::align_of::<$t>(),
        );
    };
}

/// Mach-O prefixes C-visible symbol names with an underscore.
#[cfg(target_vendor = "apple")]
#[macro_export]
macro_rules! export_app_interface_size {
    ($t:ty) => {
        core::arch::global_asm!(
            ".globl _PICTORUS_APP_INTERFACE_SIZE",
            ".set _PICTORUS_APP_INTERFACE_SIZE, {size}",
            ".globl _PICTORUS_APP_INTERFACE_ALIGN",
            ".set _PICTORUS_APP_INTERFACE_ALIGN, {align}",
            size = const core::mem::size_of::<$t>(),
            align = const core::mem::align_of::<$t>(),
        );
    };
}
