//! Discrete pins, driven through the IOPORT vtable.

use embedded_hal::digital::{ErrorType, InputPin, OutputPin};
use pictorus_blocks::{GpioInputBlockParams, GpioOutputBlockParams};
use pictorus_traits::{InputBlock, ModelClock, OutputBlock, PassBy};
use renesas_fsp_sys::{bsp_io_level_t, bsp_io_port_pin_t, ioport_instance_t};

use crate::diag::warn_once;
use crate::error::{FspError, Result, check};

/// A pin identity, in FSP's `(port << 8) | pin` encoding.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FspPin(bsp_io_port_pin_t);

impl FspPin {
    pub const fn from_port_pin(port: u8, pin: u8) -> Self {
        Self(bsp_io_port_pin_t(((port as u16) << 8) | pin as u16))
    }

    /// Wrap a `BSP_IO_PORT_xx_PIN_yy` value that C computed.
    pub const fn from_raw(value: u16) -> Self {
        Self(bsp_io_port_pin_t(value))
    }

    pub const fn raw(self) -> u16 {
        self.0.0
    }
}

/// The IOPORT driver.
///
/// `Copy`, because IOPORT is a singleton. Unlike every other peripherals, IOPORT
/// is never opened by Rust. FSP opens it during startup: `R_BSP_WarmStart(BSP_WARM_START_POST_C)`
/// calls `R_IOPORT_Open(&IOPORT_CFG_CTRL, &IOPORT_CFG_NAME)` before `.init_array`
/// runs, let alone `main`. Every pin is already at its `pin_data.c`
/// configuration by the time any of this code executes.
#[derive(Debug, Copy, Clone)]
pub struct Ioport {
    instance: *const ioport_instance_t,
}

impl Ioport {
    /// # Safety
    ///
    /// `instance` must point at a live, already-opened `ioport_instance_t` --
    /// in practice the `g_ioport` that the configurator emits into
    /// `ra_gen/hal_data.c` -- and must outlive every pin derived from it.
    /// Because that object is a `const` global in the generated C, the lifetime
    /// requirement is satisfied for the whole program.
    pub unsafe fn from_instance(instance: *const ioport_instance_t) -> Result<Self> {
        let port = Self { instance };
        // A null p_api would fault on the first pin access, a long way from the
        // configuration mistake that caused it.
        port.api()?;
        Ok(port)
    }

    fn api(&self) -> Result<&renesas_fsp_sys::ioport_api_t> {
        // SAFETY: from_instance's contract.
        unsafe { self.instance.as_ref() }
            .and_then(|instance| unsafe { instance.p_api.as_ref() })
            .ok_or(FspError::NULL_INSTANCE)
    }

    pub fn read(&self, pin: FspPin) -> Result<bool> {
        let api = self.api()?;
        let read = api.pinRead.ok_or(FspError::UNIMPLEMENTED)?;
        let mut level = bsp_io_level_t::BSP_IO_LEVEL_LOW;
        // SAFETY: from_instance's contract; `level` is a valid out-pointer.
        check(unsafe { read((*self.instance).p_ctrl, pin.0, &mut level) })?;
        Ok(level == bsp_io_level_t::BSP_IO_LEVEL_HIGH)
    }

    pub fn write(&self, pin: FspPin, high: bool) -> Result<()> {
        let api = self.api()?;
        let write = api.pinWrite.ok_or(FspError::UNIMPLEMENTED)?;
        let level = if high {
            bsp_io_level_t::BSP_IO_LEVEL_HIGH
        } else {
            bsp_io_level_t::BSP_IO_LEVEL_LOW
        };
        // SAFETY: from_instance's contract.
        check(unsafe { write((*self.instance).p_ctrl, pin.0, level) })
    }
}

/// A pin read once per tick.
#[derive(Debug)]
pub struct FspInputPin {
    ioport: Ioport,
    pin: FspPin,
    last: bool,
}

impl FspInputPin {
    pub fn new(ioport: Ioport, pin: FspPin) -> Self {
        Self {
            ioport,
            pin,
            last: false,
        }
    }
}

/// A pin written from the model.
///
/// Pins cannot be configured under this interface, this should be done by
/// the FSP tool, which ultimately resides in <fsp_prj>/ra_gen/pin_data.c.
///
/// The accepted consequence is that a pin a misconfigured output pin will
/// drive nothing, with no indication to the user.
#[derive(Debug)]
pub struct FspOutputPin {
    ioport: Ioport,
    pin: FspPin,
}

impl FspOutputPin {
    pub fn new(ioport: Ioport, pin: FspPin) -> Self {
        Self { ioport, pin }
    }
}

impl ErrorType for FspInputPin {
    type Error = FspError;
}

impl ErrorType for FspOutputPin {
    type Error = FspError;
}

impl InputPin for FspInputPin {
    fn is_high(&mut self) -> Result<bool> {
        self.ioport.read(self.pin)
    }

    fn is_low(&mut self) -> Result<bool> {
        self.is_high().map(|high| !high)
    }
}

impl OutputPin for FspOutputPin {
    fn set_high(&mut self) -> Result<()> {
        self.ioport.write(self.pin, true)
    }

    fn set_low(&mut self) -> Result<()> {
        self.ioport.write(self.pin, false)
    }
}

impl InputBlock for FspInputPin {
    type Output = f64;
    type Parameters = GpioInputBlockParams;

    fn input(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn ModelClock,
    ) -> PassBy<'_, Self::Output> {
        // A failed read holds the previous level rather than some default.
        match self.ioport.read(self.pin) {
            Ok(level) => self.last = level,
            Err(err) => warn_once!("GPIO read failed on {:?}: {}", self.pin, err),
        }
        self.last.into()
    }
}

impl OutputBlock for FspOutputPin {
    type Inputs = bool;
    type Parameters = GpioOutputBlockParams;

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn ModelClock,
        inputs: PassBy<'_, Self::Inputs>,
    ) {
        if let Err(err) = self.ioport.write(self.pin, inputs) {
            warn_once!("GPIO write failed on {:?}: {}", self.pin, err);
        }
    }
}
