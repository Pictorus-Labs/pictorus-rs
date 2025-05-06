// gpio protocols
pub use embedded_hal::digital::{InputPin, OutputPin};

// i2c protocol
pub use embedded_hal::i2c::I2c;

// pwm protocol
pub use embedded_hal_02::Pwm;

// serial protocol
pub use embedded_io::{Read, Write};

// clock protocol
pub use embedded_time::Clock;

// CAN protocol
pub use embedded_can::{Frame, nb::Can};

/// Tolerance for PWM period up to 10MHz
pub const PWM_PERIOD_TOLERANCE_POINT_1_US: f64 = 1e-7; // .1 microseconds
/// Tolerance for PWM Duty Cycle up to 16 bit - STM32
pub const PWM_DUTY_CYCLE_TOLERANCE_16_BIT: f64 = 1. / 65536.; // 16 bit PWM precision
/// Tolerance for PWM Duty Cycle up to 12 bit - Raspberry Pi
pub const PWM_DUTY_CYCLE_TOLERANCE_12_BIT: f64 = 1. / 4096.; // 12 bit PWM precision
pub const BUFF_SIZE_BYTES: usize = 1024;

// TODO: These trait wrappers are kind of dumb. But to move away from them we need to handle
// buffering in the main file rather than in the protocol implementations.
pub trait CanProtocol: Can {
    fn read_frames(&mut self) -> &[impl Frame];

    fn flush(&mut self);
}

#[cfg(feature = "std")]
pub trait UdpProtocol {
    fn read(&mut self) -> Result<&[u8], std::io::Error>;
    fn write(&mut self, buf: &[u8], to_addr: &str) -> Result<usize, std::io::Error>;
    fn flush(&mut self);
}

pub trait Flush {
    fn flush(&mut self);
}
