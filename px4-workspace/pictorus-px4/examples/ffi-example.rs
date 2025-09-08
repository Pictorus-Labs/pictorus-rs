#![no_std]
#![no_main]

use once_cell::sync::Lazy;
#[cfg(target_arch = "arm")]
use panic_halt as _;
use pictorus_px4::{
    ffi_protocol::*,
    message_impls::{SensorAccel, SensorGyro, VehicleAttitudeSetpoint},
};
use pictorus_traits::{InputBlock, Matrix, OutputBlock, Pass};

use spin::RwLock;

#[global_allocator]
static HEAP: embedded_alloc::Heap = embedded_alloc::Heap::empty();

#[derive(Default)]
pub struct StubContext;
impl pictorus_traits::Context for StubContext {
    fn fundamental_timestep(&self) -> core::time::Duration {
        core::time::Duration::from_millis(10)
    }

    fn time(&self) -> core::time::Duration {
        core::time::Duration::from_millis(100)
    }

    fn timestep(&self) -> Option<core::time::Duration> {
        Some(core::time::Duration::from_millis(10))
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn init_rust() {
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 20_480;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(core::ptr::addr_of_mut!(HEAP_MEM) as usize, HEAP_SIZE) }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn step_rust() {
    PICTORUS_MODEL.write().update();
}

static PICTORUS_MODEL: Lazy<RwLock<PictorusModel>> =
    Lazy::new(|| RwLock::new(PictorusModel::new()));

pub struct PictorusModel {
    count: usize,
    accel_input_params: FfiBlockParameters,
    accel_input_block: FfiInputBlock<SensorAccel>,
    gyro_input_params: FfiBlockParameters,
    gyro_input_block: FfiInputBlock<SensorGyro>,
    attitude_output_params: FfiBlockParameters,
    attitude_output_block: FfiOutputBlock<VehicleAttitudeSetpoint>,
}
unsafe impl Send for PictorusModel {}
unsafe impl Sync for PictorusModel {}

impl PictorusModel {
    pub fn new() -> Self {
        FfiProtocol::get_mut().subscribe_to_message(SensorAccel);
        FfiProtocol::get_mut().subscribe_to_message(SensorGyro);
        FfiProtocol::get_mut().advertise_message(VehicleAttitudeSetpoint);
        PictorusModel {
            count: 0,
            accel_input_params: FfiBlockParameters,
            accel_input_block: FfiInputBlock::default(),
            gyro_input_params: FfiBlockParameters,
            gyro_input_block: FfiInputBlock::default(),
            attitude_output_params: FfiBlockParameters,
            attitude_output_block: FfiOutputBlock::default(),
        }
    }

    fn update(&mut self) {
        let accel_data = self
            .accel_input_block
            .input(&self.accel_input_params, &StubContext::default());
        let gyro_data = self
            .gyro_input_block
            .input(&self.gyro_input_params, &StubContext::default());

        let attitude_input = (
            self.count as f64,
            42.0,
            Matrix::zeroed(),
            Matrix::zeroed(),
            accel_data.3,
            gyro_data.3,
        );
        self.attitude_output_block.output(
            &self.attitude_output_params,
            &StubContext::default(),
            attitude_input.as_by(),
        );
        self.count += 1;
    }
}
