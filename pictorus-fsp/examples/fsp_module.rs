//! A minimal stand-in for a generated Pictorus application, built as the
//! `staticlib` that an FSP project links.

// `no_std` only where there is no OS. A host `cargo test` builds every example
// as an ordinary target, and a `no_std` staticlib on a hosted target needs an
// unwinding personality routine it has no way to supply. Linking against std on
// the host costs nothing here -- the target build, which is the one that has to
// be right, is unaffected.
#![cfg_attr(target_os = "none", no_std)]

use pictorus_fsp::{FspOutputPin, FspPwm, Peripherals};

#[global_allocator]
static HEAP: embedded_alloc::Heap = embedded_alloc::Heap::empty();

/// Apps can use `panic-halt` or `panic-probe`, either is fine.
#[cfg(target_os = "none")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

/// Manages IO Blocks and peripherals
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

/// # Safety
///
/// `base` must point at `bytes` of writable memory valid for the life of the
/// program, and this must run before the first allocation. The shim guarantees
/// both: it calls this before `app_interface_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pictorus_rt_heap_init(base: *mut core::ffi::c_void, bytes: usize) {
    unsafe { HEAP.init(base as usize, bytes) }
}

#[unsafe(no_mangle)]
pub extern "C" fn pictorus_rt_build_id() -> *const core::ffi::c_char {
    c"fsp_module example".as_ptr()
}

extern crate alloc;
