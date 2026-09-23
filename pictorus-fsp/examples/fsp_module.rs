//! A minimal stand-in for a generated Pictorus application, built as the
//! `staticlib` that an FSP project links.
//!
//! Everything below that a real application also needs is marked `TEMPLATE`.
//! A Pictorus `LibType.STATIC` export already emits `app_interface_new`,
//! `app_interface_update` and `app_interface_free`; what it does not yet emit,
//! and what has to be hand-added until codegen learns to, is
//! `pictorus_rt_heap_init`, `pictorus_rt_build_id`, and an `IoManager` built
//! from [`Peripherals`] instead of `embassy_stm32::init`, these are tested here.
//!
//! `pictorus-fsp` is an rlib, and under LTO an rlib holds bitcode rather than ELF,
//! so nothing about the final object is observable until something links a `staticlib`:
//!
//! ```text
//! cargo build --example fsp_module --release --all-features \
//!     --target thumbv8m.main-none-eabihf
//! ```
//!
//! `rm_pictorus_app.c` calls and links six things and fails if any is missing:
//!
//! | Symbol | Comes from |
//! |---|---|
//! | `pictorus_rt_bind` | `pictorus-fsp` |
//! | `pictorus_rt_heap_init` | the application crate — `TEMPLATE` below |
//! | `pictorus_rt_build_id` | the application crate — `TEMPLATE` below |
//! | `app_interface_new` | Pictorus codegen, for `LibType.STATIC` |
//! | `app_interface_update` | Pictorus codegen |
//! | `app_interface_free` | Pictorus codegen |

// `no_std` only where there is no OS. A host `cargo test` builds every example
// as an ordinary target, and a `no_std` staticlib on a hosted target needs an
// unwinding personality routine it has no way to supply. Linking against std on
// the host costs nothing here -- the target build, which is the one that has to
// be right, is unaffected.
#![cfg_attr(target_os = "none", no_std)]

use pictorus_fsp::{FspOutputPin, FspPwm, Peripherals};

/// TEMPLATE. The same allocator a generated Pictorus application already uses;
/// only where it is initialised changes.
#[global_allocator]
static HEAP: embedded_alloc::Heap = embedded_alloc::Heap::empty();

/// TEMPLATE. Under `panic = "abort"` a panic in a control loop is a reset. A
/// generated application uses `panic-halt` or `panic-probe`; either is fine.
#[cfg(target_os = "none")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

/// Stands in for the generated `IoManager`.
///
/// The only structural difference from the STM32 export is where the pins come
/// from. That version opens with
///
/// ```ignore
/// let p = embassy_stm32::init(embassy_stm32::Config::default());
/// let inner = Output::new(p.PA1, Level::Low, Speed::Low);
/// let gpio_output_protocol_7473bfd0c5 = Stm32OutputPin::new(inner);
/// ```
///
/// Here there is no `init` and no pin constant: FSP brought the clocks and pins
/// up before `main`, and which physical pin each block drives was chosen in the
/// configurator. The index is the block's position in model I/O order.
struct IoManager {
    led_0: FspOutputPin,
    led_1: FspOutputPin,
    pwm_0: FspPwm,
}

impl IoManager {
    fn new() -> pictorus_fsp::Result<Self> {
        let peripherals = Peripherals::take()?;
        Ok(Self {
            led_0: peripherals.gpio_output(0)?,
            led_1: peripherals.gpio_output(1)?,
            pwm_0: peripherals.pwm(0)?,
        })
    }

    /// TEMPLATE. Generated code emits a bare `let _ = self.<x>.flush();` per
    /// input. GPIO outputs and PWM outputs have nothing to flush.
    fn flush_inputs(&mut self) {}
}

/// Stands in for `pictorus_internal::RuntimeContext`, which a generated
/// application owns instead.
struct TickClock {
    time: core::time::Duration,
}

impl pictorus_traits::ModelClock for TickClock {
    fn timestep(&self) -> Option<core::time::Duration> {
        Some(self.fundamental_timestep())
    }

    fn time(&self) -> core::time::Duration {
        self.time
    }

    fn fundamental_timestep(&self) -> core::time::Duration {
        core::time::Duration::from_millis(1)
    }
}

/// Stands in for the generated `AppInterface`. A real one owns the model's
/// `Application`, its parameters and a `pictorus_internal::RuntimeContext`.
pub struct AppInterface {
    io_manager: IoManager,
    ticks: u64,
}

/// TEMPLATE. Codegen emits this; the only edit is deleting the inline
/// `HEAP.init(...)` block it currently opens with, because the shim has already
/// called `pictorus_rt_heap_init` by the time this runs.
///
/// Returning null tells `open()` the model could not be constructed, which it
/// reports as `FSP_ERR_OUT_OF_MEMORY`. A generated version calls
/// `.expect("Unable to initialize IoManager!")` instead; on a board a null
/// return that `open()` reports is more useful than a panic that aborts.
#[unsafe(no_mangle)]
pub extern "C" fn app_interface_new() -> *mut AppInterface {
    let Ok(io_manager) = IoManager::new() else {
        return core::ptr::null_mut();
    };
    alloc::boxed::Box::into_raw(alloc::boxed::Box::new(AppInterface {
        io_manager,
        ticks: 0,
    }))
}

/// TEMPLATE, unmodified from codegen. A generated version reads
///
/// ```ignore
/// app_interface.context.update_app_time(s_to_us(app_time_s));
/// app_interface.update();
/// ```
///
/// `app_time_s` is measured by the shim from the time-base timer, so
/// `RuntimeContext` sees true elapsed time and `ModelClock::timestep()` reports a
/// real delta even when a tick overruns.
///
/// # Safety
///
/// `app` must be a pointer returned by [`app_interface_new`] and not yet freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn app_interface_update(app: *mut AppInterface, app_time_s: f64) {
    let Some(app) = (unsafe { app.as_mut() }) else {
        return;
    };

    app.ticks += 1;

    // A real application runs the model here. This one blinks and sweeps a duty
    // cycle, so that a first bring-up has something to look at on a scope.
    use embedded_hal::digital::OutputPin;
    if app.ticks % 2 == 0 {
        app.io_manager.led_0.set_high().ok();
        app.io_manager.led_1.set_low().ok();
    } else {
        app.io_manager.led_0.set_low().ok();
        app.io_manager.led_1.set_high().ok();
    }

    // Reached through `OutputBlock` because that is the only interface PWM has,
    // and it is the one codegen emits a call to.
    let clock = TickClock {
        time: core::time::Duration::from_secs_f64(app_time_s.max(0.0)),
    };
    let duty = ((app.ticks % 100) as f64) / 100.0;
    pictorus_traits::OutputBlock::output(
        &mut app.io_manager.pwm_0,
        &pictorus_blocks::PwmBlockParams::new(),
        &clock,
        (1_000.0, duty, 1.0 - duty),
    );

    app.io_manager.flush_inputs();
}

/// TEMPLATE, unmodified from codegen.
///
/// # Safety
///
/// `app` must be a pointer returned by [`app_interface_new`], freed at most
/// once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn app_interface_free(app: *mut AppInterface) {
    if app.is_null() {
        return;
    }
    drop(unsafe { alloc::boxed::Box::from_raw(app) });
}

/// TEMPLATE. Hand-added; codegen does not emit this yet.
///
/// A generated app currently initialises its heap inline at the top of
/// `app_interface_new`, from a `const HEAP_SIZE` baked into the Rust. Moving it
/// here is what lets the heap size be a GUI property instead: the size is a
/// `#define` in a generated `*_cfg.h`, which this library never compiles
/// against, so C sizes the buffer and hands it over. Set the BSP's own
/// `Heap Size` to 0 so FSP does not reserve a second arena.
///
/// # Safety
///
/// `base` must point at `bytes` of writable memory valid for the life of the
/// program, and this must run before the first allocation. The shim guarantees
/// both: it calls this before `app_interface_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pictorus_rt_heap_init(base: *mut core::ffi::c_void, bytes: usize) {
    unsafe { HEAP.init(base as usize, bytes) }
}

/// TEMPLATE. Hand-added. Codegen already has this string: it is what
/// `compile_info()` returns, needing only a trailing NUL.
#[unsafe(no_mangle)]
pub extern "C" fn pictorus_rt_build_id() -> *const core::ffi::c_char {
    c"fsp_module example".as_ptr()
}

extern crate alloc;
