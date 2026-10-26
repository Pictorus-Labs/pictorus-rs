//! A hand-written stand-in for a generated `ffi_lib`.
//!
//! It exposes the `app_interface_*` entry points described in `include/app_interface.h` over a
//! small model, so the runtime and adapters in `pictorus-ffi` can be exercised from C without
//! needing the code generator. The model is arranged the way generated code is: a set of
//! blocks reaching hardware only through an `IoManager`, which here is backed by the
//! C-implemented adapters.
#![cfg_attr(target_os = "none", no_std)]

extern crate alloc;

use alloc::vec::Vec;
use core::mem::{align_of, size_of};
use core::sync::atomic::{AtomicUsize, Ordering};
use core::time::Duration;

use pictorus_blocks::{
    AdcBlockParams, DacBlockParams, GpioInputBlockParams, GpioOutputBlockParams, PwmBlockParams,
};
use pictorus_ffi::peripherals::{
    AdcInput, DacOutput, GpioInput, GpioOutput, Peripheral, PwmOutput, pictorus_adc_t,
    pictorus_dac_t, pictorus_gpio_in_t, pictorus_gpio_out_t, pictorus_peripheral_status_t,
    pictorus_pwm_t,
};
use pictorus_ffi::{
    PICTORUS_ERR_INVALID_ARG, PICTORUS_ERR_PERIPHERAL, PICTORUS_OK, export_app_interface_size,
    pictorus_status_t,
};
use pictorus_internal::protocols::Flush;
use pictorus_traits::{InputBlock, Matrix, ModelClock, OutputBlock};

const FUNDAMENTAL_TIMESTEP_US: u64 = 10_000;

static LIVE_INSTANCES: AtomicUsize = AtomicUsize::new(0);

/// Mirror of `AppPeripherals` in `app_interface.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AppPeripherals {
    pwr_btn: pictorus_gpio_in_t,
    status_led: pictorus_gpio_out_t,
    motor_pwm: pictorus_pwm_t,
    vbat: pictorus_adc_t,
    analog_out: pictorus_dac_t,
}

/// Mirror of `AppPeripheralStatus` in `app_interface.h`.
#[repr(C)]
pub struct AppPeripheralStatus {
    pwr_btn: pictorus_peripheral_status_t,
    status_led: pictorus_peripheral_status_t,
    motor_pwm: pictorus_peripheral_status_t,
    vbat: pictorus_peripheral_status_t,
    analog_out: pictorus_peripheral_status_t,
}

/// Mirror of `AppInputs` in `app_interface.h`.
#[repr(C)]
pub struct AppInputs {
    pub setpoint: f64,
}

/// Mirror of `AppOutputs` in `app_interface.h`.
#[repr(C)]
pub struct AppOutputs {
    pub output: f64,
    pub steps: u64,
    pub button: f64,
    pub vbat_a: f64,
    pub vbat_b: f64,
}

/// The shape of the generated `IoManager`: one accessor per named peripheral.
trait IoManager {
    fn pwr_btn(&mut self) -> &mut impl InputBlock<Output = f64, Parameters = GpioInputBlockParams>;
    fn status_led(
        &mut self,
    ) -> &mut impl OutputBlock<Inputs = bool, Parameters = GpioOutputBlockParams>;
    fn motor_pwm(
        &mut self,
    ) -> &mut impl OutputBlock<Inputs = (f64, f64, f64), Parameters = PwmBlockParams>;
    fn vbat(&mut self) -> &mut impl InputBlock<Output = u16, Parameters = AdcBlockParams>;
    fn analog_out(
        &mut self,
    ) -> &mut impl OutputBlock<Inputs = Matrix<1, 2, f64>, Parameters = DacBlockParams>;
    fn flush_inputs(&mut self);
}

struct FfiIoManager {
    pwr_btn: GpioInput,
    status_led: GpioOutput,
    motor_pwm: PwmOutput,
    vbat: AdcInput,
    analog_out: DacOutput,
}

impl FfiIoManager {
    /// # Safety
    ///
    /// The handles in `peripherals` must satisfy the contracts documented in `pictorus_ffi.h`.
    unsafe fn new(peripherals: &AppPeripherals) -> Self {
        // SAFETY: forwarded from this function's contract.
        unsafe {
            Self {
                pwr_btn: GpioInput::new(peripherals.pwr_btn),
                status_led: GpioOutput::new(peripherals.status_led),
                motor_pwm: PwmOutput::new(peripherals.motor_pwm),
                vbat: AdcInput::new(peripherals.vbat),
                analog_out: DacOutput::new(peripherals.analog_out),
            }
        }
    }

    fn peripheral_status(&self) -> AppPeripheralStatus {
        AppPeripheralStatus {
            pwr_btn: self.pwr_btn.status(),
            status_led: self.status_led.status(),
            motor_pwm: self.motor_pwm.status(),
            vbat: self.vbat.status(),
            analog_out: self.analog_out.status(),
        }
    }

    fn total_errors(&self) -> u64 {
        let s = self.peripheral_status();
        [s.pwr_btn, s.status_led, s.motor_pwm, s.vbat, s.analog_out]
            .iter()
            .map(|s| u64::from(s.error_count))
            .sum()
    }
}

impl IoManager for FfiIoManager {
    fn pwr_btn(&mut self) -> &mut impl InputBlock<Output = f64, Parameters = GpioInputBlockParams> {
        &mut self.pwr_btn
    }

    fn status_led(
        &mut self,
    ) -> &mut impl OutputBlock<Inputs = bool, Parameters = GpioOutputBlockParams> {
        &mut self.status_led
    }

    fn motor_pwm(
        &mut self,
    ) -> &mut impl OutputBlock<Inputs = (f64, f64, f64), Parameters = PwmBlockParams> {
        &mut self.motor_pwm
    }

    fn vbat(&mut self) -> &mut impl InputBlock<Output = u16, Parameters = AdcBlockParams> {
        &mut self.vbat
    }

    fn analog_out(
        &mut self,
    ) -> &mut impl OutputBlock<Inputs = Matrix<1, 2, f64>, Parameters = DacBlockParams> {
        &mut self.analog_out
    }

    fn flush_inputs(&mut self) {
        self.vbat.flush();
    }
}

struct TickClock {
    time: Duration,
}

impl ModelClock for TickClock {
    fn timestep(&self) -> Option<Duration> {
        Some(self.fundamental_timestep())
    }

    fn time(&self) -> Duration {
        self.time
    }

    fn fundamental_timestep(&self) -> Duration {
        Duration::from_micros(FUNDAMENTAL_TIMESTEP_US)
    }
}

/// Owns heap data so the tests cover the allocator, not only plain-old-data instances.
pub struct AppInterface {
    io_manager: FfiIoManager,
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
/// used for anything else until `app_interface_deinit`. `peripherals` must be null or valid
/// and its handles must satisfy the contracts in `pictorus_ffi.h`. `out` must be null or valid
/// for a write.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn app_interface_init(
    mem: *mut core::ffi::c_void,
    peripherals: *const AppPeripherals,
    out: *mut *mut AppInterface,
) -> pictorus_status_t {
    // SAFETY: forwarded from this function's contract.
    let Some(peripherals) = (unsafe { peripherals.as_ref() }) else {
        return PICTORUS_ERR_INVALID_ARG;
    };
    if mem.is_null() || out.is_null() || !mem.cast::<AppInterface>().is_aligned() {
        return PICTORUS_ERR_INVALID_ARG;
    }

    let app = mem.cast::<AppInterface>();
    // SAFETY: `mem` is non-null, aligned, and large enough per this function's contract, and the
    // peripheral handles are valid per the same.
    unsafe {
        app.write(AppInterface {
            io_manager: FfiIoManager::new(peripherals),
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

    let errors_before = app.io_manager.total_errors();
    let clock = TickClock {
        time: Duration::from_micros(app_time_us),
    };
    let io = &mut app.io_manager;

    let button = io.pwr_btn().input(&GpioInputBlockParams::new(), &clock);
    io.status_led()
        .output(&GpioOutputBlockParams::new(), &clock, button != 0.0);

    // Two blocks sharing one ADC, which must see the same sample.
    let vbat_a = io.vbat().input(&AdcBlockParams::new(), &clock);
    let vbat_b = io.vbat().input(&AdcBlockParams::new(), &clock);

    let duty = f64::from(vbat_a) / 4095.0;
    io.motor_pwm()
        .output(&PwmBlockParams::new(), &clock, (1000.0, duty, 1.0 - duty));
    io.analog_out().output(
        &DacBlockParams::new(),
        &clock,
        &Matrix {
            data: [[inputs.setpoint], [inputs.setpoint * 2.0]],
        },
    );
    io.flush_inputs();

    app.history.push(app_time_us);
    outputs.output = inputs.setpoint * 2.0;
    outputs.steps = app.history.len() as u64;
    outputs.button = button;
    outputs.vbat_a = f64::from(vbat_a);
    outputs.vbat_b = f64::from(vbat_b);

    if app.io_manager.total_errors() > errors_before {
        PICTORUS_ERR_PERIPHERAL
    } else {
        PICTORUS_OK
    }
}

/// # Safety
///
/// `app` must come from `app_interface_init` and not yet be deinitialized. `out` must be null
/// or valid for a write.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn app_interface_peripheral_status(
    app: *const AppInterface,
    out: *mut AppPeripheralStatus,
) -> pictorus_status_t {
    // SAFETY: forwarded from this function's contract.
    let (Some(app), Some(out)) = (unsafe { app.as_ref() }, unsafe { out.as_mut() }) else {
        return PICTORUS_ERR_INVALID_ARG;
    };
    *out = app.io_manager.peripheral_status();
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
