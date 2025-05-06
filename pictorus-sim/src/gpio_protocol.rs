use std::convert::Infallible;

use embedded_hal::digital::{ErrorType, InputPin, OutputPin};
use pictorus_blocks::{GpioInputBlockParams, GpioOutputBlockParams};
use pictorus_traits::{Context, InputBlock, OutputBlock, PassBy};

pub struct SimGpioPin {}

impl ErrorType for SimGpioPin {
    type Error = Infallible;
}

impl InputPin for SimGpioPin {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        Ok(true)
    }
    fn is_low(&mut self) -> Result<bool, Self::Error> {
        Ok(false)
    }
}

impl OutputPin for SimGpioPin {
    fn set_high(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

pub fn create_gpio_input_pin(_: f64) -> Result<SimGpioPin, Infallible> {
    Ok(SimGpioPin {})
}

pub fn create_gpio_output_pin(_: f64) -> Result<SimGpioPin, Infallible> {
    Ok(SimGpioPin {})
}

impl InputBlock for SimGpioPin {
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

impl OutputBlock for SimGpioPin {
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
