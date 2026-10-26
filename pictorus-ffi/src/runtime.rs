//! Process-wide runtime: the heap, panic handling and ABI version.
//!
//! A Rust `#[global_allocator]` and `#[panic_handler]` exist once per linked binary, and stable
//! Rust has no per-instance allocators, so every app instance in a binary shares what is set up
//! here. Only the integrator knows the total memory needed and where it should live, so they
//! hand it over through [`pictorus_rt_init`].

use core::ffi::{c_char, c_void};
use core::sync::atomic::{AtomicBool, Ordering};

use crate::status::*;

/// Version of the C interface. Must match `PICTORUS_FFI_ABI_VERSION` in `pictorus_ffi.h`.
pub const ABI_VERSION: u32 = 1;

/// Mirror of `pictorus_rt_config_t` in `pictorus_ffi.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct pictorus_rt_config_t {
    pub heap: *mut c_void,
    pub heap_bytes: usize,
    pub on_panic: Option<unsafe extern "C" fn(msg: *const c_char, len: usize)>,
    pub critical_section_enter: Option<unsafe extern "C" fn()>,
    pub critical_section_exit: Option<unsafe extern "C" fn()>,
}

impl pictorus_rt_config_t {
    const EMPTY: Self = Self {
        heap: core::ptr::null_mut(),
        heap_bytes: 0,
        on_panic: None,
        critical_section_enter: None,
        critical_section_exit: None,
    };
}

static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Initializes the runtime. See `pictorus_ffi.h`.
///
/// # Safety
///
/// `config` must be null or point to a valid [`pictorus_rt_config_t`]. If it names a heap, the
/// memory must be writable, unused by anything else, and valid for the rest of the program.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pictorus_rt_init(
    config: *const pictorus_rt_config_t,
) -> pictorus_status_t {
    // SAFETY: the caller guarantees `config` is null or valid.
    let config = unsafe { config.as_ref() }
        .copied()
        .unwrap_or(pictorus_rt_config_t::EMPTY);

    if config.heap.is_null() != (config.heap_bytes == 0) {
        return PICTORUS_ERR_INVALID_ARG;
    }

    if INITIALIZED.swap(true, Ordering::AcqRel) {
        return PICTORUS_ERR_ALREADY_INIT;
    }

    #[cfg(all(not(feature = "std"), target_os = "none"))]
    // SAFETY: the caller guarantees the heap memory is valid, and the `INITIALIZED` guard above
    // means this runs once, before any app instance exists to allocate.
    unsafe {
        hooks::install(&config)
    };

    PICTORUS_OK
}

/// Returns [`ABI_VERSION`] as this library was built.
#[unsafe(no_mangle)]
pub extern "C" fn pictorus_rt_abi_version() -> u32 {
    ABI_VERSION
}

/// Allocator, critical section and panic handler. Only present where there is no OS to supply
/// them. The `target_os = "none"` condition (rather than only `not(feature = "std")`) keeps host
/// builds and test binaries, which link the standard library, free of duplicate lang items.
#[cfg(all(not(feature = "std"), target_os = "none"))]
mod hooks {
    use super::pictorus_rt_config_t;
    use core::ffi::c_char;
    use core::fmt::Write;
    use core::sync::atomic::{AtomicPtr, Ordering};

    type PanicFn = unsafe extern "C" fn(*const c_char, usize);
    type CsFn = unsafe extern "C" fn();

    // Function pointers held as `*mut ()` so they can live in atomics.
    static ON_PANIC: AtomicPtr<()> = AtomicPtr::new(core::ptr::null_mut());
    static CS_ENTER: AtomicPtr<()> = AtomicPtr::new(core::ptr::null_mut());
    static CS_EXIT: AtomicPtr<()> = AtomicPtr::new(core::ptr::null_mut());

    #[cfg(feature = "alloc")]
    #[global_allocator]
    static HEAP: embedded_alloc::Heap = embedded_alloc::Heap::empty();

    /// # Safety
    ///
    /// Must be called once, before anything allocates. A non-null `config.heap` must be valid
    /// for `config.heap_bytes` bytes, for the rest of the program.
    pub(super) unsafe fn install(config: &pictorus_rt_config_t) {
        ON_PANIC.store(erase(config.on_panic), Ordering::Release);
        CS_ENTER.store(erase(config.critical_section_enter), Ordering::Release);
        CS_EXIT.store(erase(config.critical_section_exit), Ordering::Release);

        #[cfg(feature = "alloc")]
        if !config.heap.is_null() {
            // SAFETY: forwarded from this function's contract.
            unsafe { HEAP.init(config.heap as usize, config.heap_bytes) };
        }
    }

    fn erase<F: Copy>(f: Option<F>) -> *mut () {
        const { assert!(size_of::<F>() == size_of::<*mut ()>()) };
        // SAFETY: `F` is a function pointer, checked above to be pointer sized.
        f.map_or(core::ptr::null_mut(), |f| unsafe {
            core::mem::transmute_copy::<F, *mut ()>(&f)
        })
    }

    struct CriticalSection;
    critical_section::set_impl!(CriticalSection);

    // SAFETY: mutual exclusion is exactly as strong as the integrator's callbacks. With none
    // provided it is a no-op, which `pictorus_ffi.h` documents as requiring a single calling
    // context.
    unsafe impl critical_section::Impl for CriticalSection {
        unsafe fn acquire() -> critical_section::RawRestoreState {
            call(&CS_ENTER);
        }

        unsafe fn release(_: critical_section::RawRestoreState) {
            call(&CS_EXIT);
        }
    }

    fn call(slot: &AtomicPtr<()>) {
        let f = slot.load(Ordering::Acquire);
        if !f.is_null() {
            // SAFETY: only ever stored from an `Option<CsFn>` in `install`.
            unsafe { core::mem::transmute::<*mut (), CsFn>(f)() };
        }
    }

    /// Fixed-size sink so a panic message can be produced without allocating.
    struct MessageBuf {
        buf: [u8; 128],
        len: usize,
    }

    impl Write for MessageBuf {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let n = s.len().min(self.buf.len() - self.len);
            self.buf[self.len..self.len + n].copy_from_slice(&s.as_bytes()[..n]);
            self.len += n;
            Ok(())
        }
    }

    #[panic_handler]
    fn panic(info: &core::panic::PanicInfo) -> ! {
        let f = ON_PANIC.load(Ordering::Acquire);
        if !f.is_null() {
            let mut msg = MessageBuf {
                buf: [0; 128],
                len: 0,
            };
            // Writing to `MessageBuf` never fails, it truncates.
            let _ = write!(msg, "{info}");
            // SAFETY: only ever stored from an `Option<PanicFn>` in `install`.
            unsafe {
                core::mem::transmute::<*mut (), PanicFn>(f)(msg.buf.as_ptr().cast(), msg.len)
            };
        }
        loop {
            core::hint::spin_loop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::format;

    fn heap_config(heap: *mut c_void, heap_bytes: usize) -> pictorus_rt_config_t {
        pictorus_rt_config_t {
            heap,
            heap_bytes,
            ..pictorus_rt_config_t::EMPTY
        }
    }

    fn header_define(name: &str) -> i32 {
        let prefix = format!("#define {name} ");
        include_str!("../include/pictorus_ffi.h")
            .lines()
            .find_map(|line| line.strip_prefix(prefix.as_str()))
            .unwrap_or_else(|| panic!("{name} not defined in header"))
            .trim_matches(|c| c == '(' || c == ')')
            .parse()
            .unwrap()
    }

    #[test]
    fn header_agrees_with_rust() {
        let expected = [
            ("PICTORUS_FFI_ABI_VERSION", ABI_VERSION as i32),
            ("PICTORUS_OK", PICTORUS_OK),
            ("PICTORUS_EMPTY", PICTORUS_EMPTY),
            ("PICTORUS_ERR_PERIPHERAL", PICTORUS_ERR_PERIPHERAL),
            ("PICTORUS_ERR_INVALID_ARG", PICTORUS_ERR_INVALID_ARG),
            ("PICTORUS_ERR_ALREADY_INIT", PICTORUS_ERR_ALREADY_INIT),
        ];
        for (name, value) in expected {
            assert_eq!(header_define(name), value, "{name}");
        }
    }

    #[test]
    fn rejects_inconsistent_heap() {
        let mut byte = 0u8;
        let heap = (&mut byte as *mut u8).cast();
        assert_eq!(
            unsafe { pictorus_rt_init(&heap_config(heap, 0)) },
            PICTORUS_ERR_INVALID_ARG
        );
        assert_eq!(
            unsafe { pictorus_rt_init(&heap_config(core::ptr::null_mut(), 16)) },
            PICTORUS_ERR_INVALID_ARG
        );
    }

    // The flag is process-wide, so both outcomes are checked in order in one test.
    #[test]
    fn init_succeeds_once() {
        assert_eq!(unsafe { pictorus_rt_init(core::ptr::null()) }, PICTORUS_OK);
        assert_eq!(
            unsafe { pictorus_rt_init(core::ptr::null()) },
            PICTORUS_ERR_ALREADY_INIT
        );
    }

    #[test]
    fn abi_version_matches() {
        assert_eq!(pictorus_rt_abi_version(), ABI_VERSION);
    }
}
