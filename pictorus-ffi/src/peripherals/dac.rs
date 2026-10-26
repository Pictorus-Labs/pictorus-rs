use core::ffi::c_void;

use pictorus_blocks::DacBlockParams;
use pictorus_traits::{Matrix, ModelClock, OutputBlock, PassBy};

use super::{Peripheral, pictorus_peripheral_status_t};
use crate::status::{PICTORUS_ERR_PERIPHERAL, pictorus_status_t};

/// Mirror of `pictorus_dac_t` in `pictorus_ffi.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct pictorus_dac_t {
    pub ctx: *mut c_void,
    pub write: Option<
        unsafe extern "C" fn(ctx: *mut c_void, values: *const f64, n: usize) -> pictorus_status_t,
    >,
}

/// A DAC output backed by C.
///
/// The model's values are passed through unchanged, in channel order. Converting them to
/// whatever the hardware expects is up to the C side.
pub struct DacOutput {
    handle: pictorus_dac_t,
    status: pictorus_peripheral_status_t,
}

impl DacOutput {
    /// # Safety
    ///
    /// `handle.ctx` must remain valid for as long as the adapter lives, and `handle.write`, if
    /// set, must be sound to call with it.
    pub unsafe fn new(handle: pictorus_dac_t) -> Self {
        Self {
            handle,
            status: Default::default(),
        }
    }
}

impl Peripheral for DacOutput {
    fn status(&self) -> pictorus_peripheral_status_t {
        self.status
    }
}

impl OutputBlock for DacOutput {
    type Inputs = Matrix<1, 2, f64>;
    type Parameters = DacBlockParams;

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _model_clock: &dyn ModelClock,
        inputs: PassBy<'_, Self::Inputs>,
    ) {
        let values = inputs.data.as_flattened();
        let status = match self.handle.write {
            // SAFETY: guaranteed by the contract of `new`, and `values` is valid for the call.
            Some(write) => unsafe { write(self.handle.ctx, values.as_ptr(), values.len()) },
            None => PICTORUS_ERR_PERIPHERAL,
        };
        self.status.record(status);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peripherals::test_util::Clock;
    use crate::status::PICTORUS_OK;
    use std::vec::Vec;

    #[derive(Default)]
    struct Fake {
        written: Vec<f64>,
        result: pictorus_status_t,
    }

    unsafe extern "C" fn write(
        ctx: *mut c_void,
        values: *const f64,
        n: usize,
    ) -> pictorus_status_t {
        let fake = unsafe { &mut *(ctx as *mut Fake) };
        fake.written = unsafe { core::slice::from_raw_parts(values, n) }.to_vec();
        fake.result
    }

    #[test]
    fn passes_channels_in_order() {
        let mut fake = Fake::default();
        let mut dac = unsafe {
            DacOutput::new(pictorus_dac_t {
                ctx: (&mut fake as *mut Fake).cast(),
                write: Some(write),
            })
        };
        // Matrix is column-major, so each column is one channel.
        let values = Matrix::<1, 2, f64> {
            data: [[1.5], [2.5]],
        };
        dac.output(&DacBlockParams::new(), &Clock, &values);
        assert_eq!(fake.written, [1.5, 2.5]);
        assert_eq!(dac.status(), Default::default());
    }

    #[test]
    fn failure_is_recorded() {
        let mut fake = Fake {
            result: PICTORUS_ERR_PERIPHERAL,
            ..Default::default()
        };
        let mut dac = unsafe {
            DacOutput::new(pictorus_dac_t {
                ctx: (&mut fake as *mut Fake).cast(),
                write: Some(write),
            })
        };
        dac.output(&DacBlockParams::new(), &Clock, &Matrix::zeroed());
        assert_eq!(dac.status().error_count, 1);
        assert_ne!(fake.result, PICTORUS_OK);
    }

    #[test]
    fn missing_function_is_a_counted_failure() {
        let mut dac = unsafe {
            DacOutput::new(pictorus_dac_t {
                ctx: core::ptr::null_mut(),
                write: None,
            })
        };
        dac.output(&DacBlockParams::new(), &Clock, &Matrix::zeroed());
        assert_eq!(dac.status().error_count, 1);
    }
}
