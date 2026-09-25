//! FSP error codes as a Rust error type.

use core::fmt;

use renesas_fsp_sys::fsp_err_t;

pub type Result<T> = core::result::Result<T, FspError>;

/// An `fsp_err_t` returned by an FSP driver.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct FspError(pub fsp_err_t);

impl FspError {
    pub const fn code(self) -> u32 {
        self.0.0
    }

    /// Named codes worth distinguishing at a call site. Everything else is
    /// reported by number.
    fn name(self) -> Option<&'static str> {
        Some(match self.0 {
            fsp_err_t::FSP_ERR_ASSERTION => "ASSERTION",
            fsp_err_t::FSP_ERR_INVALID_POINTER => "INVALID_POINTER",
            fsp_err_t::FSP_ERR_INVALID_ARGUMENT => "INVALID_ARGUMENT",
            fsp_err_t::FSP_ERR_INVALID_CHANNEL => "INVALID_CHANNEL",
            fsp_err_t::FSP_ERR_INVALID_MODE => "INVALID_MODE",
            fsp_err_t::FSP_ERR_UNSUPPORTED => "UNSUPPORTED",
            fsp_err_t::FSP_ERR_NOT_OPEN => "NOT_OPEN",
            fsp_err_t::FSP_ERR_IN_USE => "IN_USE",
            fsp_err_t::FSP_ERR_ALREADY_OPEN => "ALREADY_OPEN",
            fsp_err_t::FSP_ERR_ABORTED => "ABORTED",
            fsp_err_t::FSP_ERR_TIMEOUT => "TIMEOUT",
            fsp_err_t::FSP_ERR_INVALID_STATE => "INVALID_STATE",
            fsp_err_t::FSP_ERR_IRQ_BSP_DISABLED => "IRQ_BSP_DISABLED",
            fsp_err_t::FSP_ERR_IP_CHANNEL_NOT_PRESENT => "IP_CHANNEL_NOT_PRESENT",
            _ => return None,
        })
    }
}

impl fmt::Debug for FspError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.name() {
            Some(name) => write!(f, "FSP_ERR_{name}"),
            None => write!(f, "fsp_err_t({})", self.code()),
        }
    }
}

/// Errors this crate raises on its own behalf, where no FSP call failed.
///
/// Mapped onto FSP codes rather than given a separate type so that every
/// fallible entry point in the crate has one error type. The choices are the
/// closest existing meanings, not inventions.
impl FspError {
    /// A vtable entry FSP declares as optional was null on the instance that
    /// was handed over.
    pub const UNIMPLEMENTED: Self = Self(fsp_err_t::FSP_ERR_UNSUPPORTED);

    /// A null `*_instance_t`, or an instance with a null `p_api`/`p_ctrl`.
    pub const NULL_INSTANCE: Self = Self(fsp_err_t::FSP_ERR_INVALID_POINTER);

    /// A timer reported a zero counter frequency from `infoGet`
    pub const INVALID_CLOCK: Self = Self(fsp_err_t::FSP_ERR_INVALID_HW_CONDITION);

    /// [`crate::Peripherals::take`] was called twice. Two owners of one
    /// peripheral would each believe they had exclusive use of it.
    pub const ALREADY_TAKEN: Self = Self(fsp_err_t::FSP_ERR_ALREADY_OPEN);

    /// I/O was constructed before the shim called `pictorus_rt_bind`.
    pub const NOT_BOUND: Self = Self(fsp_err_t::FSP_ERR_NOT_INITIALIZED);

    /// The application asked for a peripheral index the generated table does
    /// not have
    pub const NO_SUCH_BLOCK: Self = Self(fsp_err_t::FSP_ERR_NOT_FOUND);
}

/// Turn an `fsp_err_t` returned across FFI into a `Result`.
pub(crate) fn check(err: fsp_err_t) -> Result<()> {
    if err == fsp_err_t::FSP_SUCCESS {
        Ok(())
    } else {
        Err(FspError(err))
    }
}

impl embedded_hal::digital::Error for FspError {
    fn kind(&self) -> embedded_hal::digital::ErrorKind {
        embedded_hal::digital::ErrorKind::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The named codes are matched against `fsp_err_t` constants rather than
    /// against numbers, so this mostly guards the fallback: an unlisted code
    /// must still be reportable, because FSP can return one these bindings have
    /// never seen.
    #[test]
    fn unlisted_codes_report_by_number() {
        extern crate std;
        use std::format;

        assert_eq!(
            format!("{:?}", FspError(fsp_err_t::FSP_ERR_TIMEOUT)),
            "FSP_ERR_TIMEOUT"
        );
        assert_eq!(
            format!("{:?}", FspError(fsp_err_t(0xDEAD))),
            "fsp_err_t(57005)"
        );
    }

    /// `check` is the only place an `fsp_err_t` becomes a `Result`, and every
    /// unsafe vtable call in the crate goes through it.
    #[test]
    fn only_success_is_ok() {
        assert!(check(fsp_err_t::FSP_SUCCESS).is_ok());
        assert_eq!(
            check(fsp_err_t::FSP_ERR_NOT_OPEN),
            Err(FspError(fsp_err_t::FSP_ERR_NOT_OPEN))
        );
    }
}
