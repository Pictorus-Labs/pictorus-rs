pub use embedded_hal::digital::{ErrorType, InputPin, OutputPin};
use linux_embedded_hal::gpio_cdev::{Chip, LineRequestFlags};
use pictorus_blocks::{GpioInputBlockParams, GpioOutputBlockParams};
use pictorus_internal::utils::PictorusError;
use pictorus_traits::{Context, InputBlock, OutputBlock, PassBy};

// TODO: This should be configurable by block param
const GPIO_CHIP: &str = "/dev/gpiochip0";
const ERR_TYPE: &str = "GpioProtocol";

pub struct CdevPin(linux_embedded_hal::CdevPin);
impl CdevPin {
    pub fn new(inner: linux_embedded_hal::CdevPin) -> Result<Self, PictorusError> {
        Ok(CdevPin(inner))
    }
}

impl ErrorType for CdevPin {
    type Error = <linux_embedded_hal::CdevPin as ErrorType>::Error;
}

impl InputPin for CdevPin {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        self.0.is_high()
    }

    fn is_low(&mut self) -> Result<bool, Self::Error> {
        self.0.is_low()
    }
}

impl OutputPin for CdevPin {
    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.0.set_high()
    }

    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.0.set_low()
    }
}

fn create_error(message: String) -> PictorusError {
    PictorusError::new(ERR_TYPE.into(), message)
}

fn create_pin_error(pin: u32) -> PictorusError {
    create_error(format!("Failed to bind to GPIO pin: {}", pin))
}

fn create_cdev_pin(
    chip: &str,
    pin_line: f64,
    flag: LineRequestFlags,
) -> Result<CdevPin, PictorusError> {
    let pin_line = pin_line as u32;
    let mut chip = Chip::new(chip).map_err(|_| {
        create_error(format!(
            "Failed to bind to GPIO bus {} for pin: {}",
            chip, pin_line
        ))
    })?;
    let handle = chip
        .get_line(pin_line)
        // TODO: Might be cleaner to impl From<linux_embedded_hal::gpio_cdev::Error> for PictorusError
        .map_err(|_| create_pin_error(pin_line))?
        .request(flag, 0, "pictorus")
        .map_err(|_| create_pin_error(pin_line))?;

    let inner = linux_embedded_hal::CdevPin::new(handle).map_err(|_| create_pin_error(pin_line))?;
    Ok(CdevPin(inner))
}

pub fn create_gpio_input_pin(pin_number: f64) -> Result<CdevPin, PictorusError> {
    create_cdev_pin(GPIO_CHIP, pin_number, LineRequestFlags::INPUT)
}

pub fn create_gpio_output_pin(pin_number: f64) -> Result<CdevPin, PictorusError> {
    create_cdev_pin(GPIO_CHIP, pin_number, LineRequestFlags::OUTPUT)
}

impl InputBlock for CdevPin {
    type Output = f64;
    type Parameters = GpioInputBlockParams;

    fn input(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn Context,
    ) -> PassBy<'_, Self::Output> {
        self.is_high().unwrap_or(false).into()
    }
}

impl OutputBlock for CdevPin {
    type Inputs = bool;
    type Parameters = GpioOutputBlockParams;

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) {
        if inputs {
            self.set_high().ok();
        } else {
            self.set_low().ok();
        }
    }
}
