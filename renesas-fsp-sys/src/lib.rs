//! Raw FFI declarations for the Renesas Flexible Software Package (FSP),
//! covering IOPORT (GPIO) and timer (PWM) from FSP 6.6.0.
//!
//! Only FSP's *interface* layer is bound -- `r_ioport_api.h` and
//! `r_timer_api.h`, which describe what a peripheral does rather than which
//! registers it does it with. The *instance* layer (`r_ioport.h`, `r_gpt.h`)
//! names device registers that do not exist until a user configures an e2
//! studio project, so it cannot be bound here.
//!
//! Two consequences are worth knowing before calling anything:
//!
//! - **Call through the vtable.** Use
//!   `instance.p_api->method(instance.p_ctrl, ...)`, never `R_IOPORT_PinWrite`
//!   or any other direct symbol. Those name one implementation and are not
//!   declared here.
//! - **The ABI is `thumbv8m.main-none-eabihf` with `-fshort-enums`,** matching
//!   how Renesas builds FSP. Deviating from either is a silent mismatch rather
//!   than a compile error.
//!
//! Nothing here links: the crate emits declarations, and the e2 studio project
//! that consumes the resulting static library resolves them.
//!
//! See `README.md` for the reasoning behind all of the above, a worked GPIO
//! example, and how to re-vendor headers or add a peripheral.

#![no_std]

/// Machine-generated; lint exemptions apply to the generator's output, not to
/// anything anyone will edit.
#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(clippy::all)]
#[allow(clippy::missing_safety_doc)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub use bindings::*;

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    /// The widths that `-fshort-enums` produces, mirrored on the Rust side.
    ///
    /// `shim/bsp_api.h` static-asserts these in C, which catches the flag being
    /// dropped from `build.rs`. This catches the other direction: a bindgen
    /// change that stops honouring the C width when emitting the newtype. Get
    /// either wrong and every field after the offending one shifts, in silence.
    #[test]
    fn bsp_enums_have_fsp_widths() {
        assert_eq!(size_of::<bsp_io_level_t>(), 1);
        assert_eq!(size_of::<bsp_io_port_t>(), 2);
        assert_eq!(size_of::<bsp_io_port_pin_t>(), 2);
        assert_eq!(size_of::<IRQn_Type>(), 1);
    }

    /// `fsp_err_t` must be a transparent newtype over an integer, never a Rust
    /// enum: FSP can return a discriminant these vendored headers have never
    /// heard of, and materialising one as a Rust enum is undefined behaviour.
    /// A newtype round-trips any value, including the 0x40000 top of the range.
    #[test]
    fn fsp_err_admits_unlisted_discriminants() {
        assert_eq!(size_of::<fsp_err_t>(), size_of::<core::ffi::c_uint>());
        assert_eq!(fsp_err_t::FSP_SUCCESS.0, 0);
        assert_eq!(fsp_err_t::FSP_ERR_COMMS_BUS_NOT_OPEN.0, 0x40000);

        // Not an enumerator anywhere in the vendored headers.
        assert_eq!(fsp_err_t(0xDEAD).0, 0xDEAD);
    }

    /// `vendor/FSP_VERSION` records which pack the headers came from, and
    /// `script/vendor-headers.sh` writes it. Nothing else checks it against the
    /// headers themselves, so a partial re-vendor could leave the two
    /// disagreeing with no visible symptom.
    #[test]
    fn recorded_pack_version_matches_vendored_headers() {
        let recorded = include_str!("../vendor/FSP_VERSION").trim();
        let mut parts = recorded.split('.').map(|p| p.parse::<u32>().unwrap());

        assert_eq!(parts.next(), Some(FSP_VERSION_MAJOR));
        assert_eq!(parts.next(), Some(FSP_VERSION_MINOR));
        assert_eq!(parts.next(), Some(FSP_VERSION_PATCH));
        assert_eq!(
            parts.next(),
            None,
            "unexpected trailing field in {recorded}"
        );
    }

    /// The vtable slots the `pictorus-fsp` GPIO and PWM implementations are
    /// built on. Present and non-null is not checkable here -- these are
    /// function-pointer *fields* -- but a pack upgrade that renames or removes
    /// one should fail here rather than in a downstream crate.
    #[test]
    fn required_vtable_slots_exist() {
        let ioport: ioport_api_t = unsafe { core::mem::zeroed() };
        let _ = (ioport.pinRead, ioport.pinWrite);

        let timer: timer_api_t = unsafe { core::mem::zeroed() };
        let _ = (timer.periodSet, timer.dutyCycleSet, timer.infoGet);

        assert_eq!(timer_mode_t::TIMER_MODE_PWM.0, 2);
    }
}
