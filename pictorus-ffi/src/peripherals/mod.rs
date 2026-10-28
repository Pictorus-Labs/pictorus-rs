//! Adapters that implement the block I/O traits over peripherals implemented in C.
//!
//! Every peripheral kind has a fixed handle type: a context pointer plus the function pointers
//! for that kind's operations. The C functions are plain "do it now" operations shaped like
//! vendor HAL calls. Everything a block expects of its protocol object beyond that, such as
//! once-per-tick caching, stays here in Rust so an app behaves the same as on a native target.
//!
//! Failures never change model behavior. A failed operation is treated as "no new data": an
//! input keeps its previous value and an output is retried on the next tick. The failure is
//! recorded in the adapter's [`pictorus_peripheral_status_t`] so the integrator can see it.

use crate::status::{PICTORUS_OK, pictorus_status_t};

pub mod adc;
pub mod dac;
pub mod gpio;
pub mod pwm;

// The adapters for byte-oriented peripherals buffer data, and the blocks that consume it are
// only available when the app allocates.
#[cfg(feature = "alloc")]
mod byte_buffer;
#[cfg(feature = "alloc")]
pub mod can;
#[cfg(feature = "alloc")]
pub mod i2c;
#[cfg(feature = "alloc")]
pub mod serial;
#[cfg(feature = "alloc")]
pub mod spi;
#[cfg(feature = "alloc")]
pub mod udp;

pub use adc::{AdcInput, pictorus_adc_t};
#[cfg(feature = "alloc")]
pub use byte_buffer::RECEIVE_BUFFER_BYTES;
#[cfg(feature = "alloc")]
pub use can::{CanBus, RX_FRAMES_PER_TICK, pictorus_can_frame_t, pictorus_can_t};
pub use dac::{DacOutput, pictorus_dac_t};
pub use gpio::{GpioInput, GpioOutput, pictorus_gpio_in_t, pictorus_gpio_out_t};
#[cfg(feature = "alloc")]
pub use i2c::{I2cBus, pictorus_i2c_t};
pub use pwm::{PwmOutput, pictorus_pwm_t};
#[cfg(feature = "alloc")]
pub use serial::{SerialPort, pictorus_serial_t};
#[cfg(feature = "alloc")]
pub use spi::{SpiBus, pictorus_spi_t};
#[cfg(feature = "alloc")]
pub use udp::{UdpPort, pictorus_udp_t};

/// Mirror of `pictorus_peripheral_status_t` in `pictorus_ffi.h`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct pictorus_peripheral_status_t {
    /// The most recent failure, or `PICTORUS_OK` if there has never been one.
    pub last_error: pictorus_status_t,
    /// Number of failed operations since init. Saturates rather than wrapping.
    pub error_count: u32,
}

impl pictorus_peripheral_status_t {
    /// Records the outcome of a C operation and reports whether it fully succeeded.
    ///
    /// Negative codes are failures. Non-negative codes other than `PICTORUS_OK` (such as
    /// `PICTORUS_EMPTY`) are not failures, but also don't carry the data the caller asked for.
    pub(crate) fn record(&mut self, status: pictorus_status_t) -> bool {
        if status < 0 {
            self.last_error = status;
            self.error_count = self.error_count.saturating_add(1);
        }
        status == PICTORUS_OK
    }
}

/// Implemented by every adapter so generated code can report per-peripheral health.
pub trait Peripheral {
    fn status(&self) -> pictorus_peripheral_status_t;
}

#[cfg(test)]
pub(crate) mod test_util {
    use core::time::Duration;
    use pictorus_traits::ModelClock;

    pub struct Clock;

    impl ModelClock for Clock {
        fn timestep(&self) -> Option<Duration> {
            Some(self.fundamental_timestep())
        }

        fn time(&self) -> Duration {
            Duration::ZERO
        }

        fn fundamental_timestep(&self) -> Duration {
            Duration::from_millis(10)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status::{PICTORUS_EMPTY, PICTORUS_ERR_PERIPHERAL};

    #[test]
    fn only_negative_codes_are_errors() {
        let mut status = pictorus_peripheral_status_t::default();

        assert!(status.record(PICTORUS_OK));
        assert!(!status.record(PICTORUS_EMPTY));
        assert_eq!(status, pictorus_peripheral_status_t::default());

        assert!(!status.record(PICTORUS_ERR_PERIPHERAL));
        assert_eq!(status.last_error, PICTORUS_ERR_PERIPHERAL);
        assert_eq!(status.error_count, 1);

        // A later success does not erase the record of the failure.
        assert!(status.record(PICTORUS_OK));
        assert_eq!(status.last_error, PICTORUS_ERR_PERIPHERAL);
        assert_eq!(status.error_count, 1);
    }

    #[test]
    fn error_count_saturates() {
        let mut status = pictorus_peripheral_status_t {
            last_error: 0,
            error_count: u32::MAX,
        };
        status.record(PICTORUS_ERR_PERIPHERAL);
        assert_eq!(status.error_count, u32::MAX);
    }
}
