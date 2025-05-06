use embedded_hal::digital::{ErrorType, InputPin, OutputPin};
use pictorus_blocks::{GpioInputBlockParams, GpioOutputBlockParams};
use pictorus_traits::{Context, InputBlock, OutputBlock, PassBy};

pub struct Stm32InputPin<'d>(embassy_stm32::gpio::Input<'d>);
impl<'d> Stm32InputPin<'d> {
    pub fn new(inner: embassy_stm32::gpio::Input<'d>) -> Self {
        Stm32InputPin(inner)
    }
}

pub struct Stm32OutputPin<'d>(embassy_stm32::gpio::Output<'d>);
impl<'d> Stm32OutputPin<'d> {
    pub fn new(inner: embassy_stm32::gpio::Output<'d>) -> Self {
        Stm32OutputPin(inner)
    }
}

impl<'d> ErrorType for Stm32InputPin<'d> {
    type Error = <embassy_stm32::gpio::Input<'d> as ErrorType>::Error;
}

impl<'d> ErrorType for Stm32OutputPin<'d> {
    type Error = <embassy_stm32::gpio::Output<'d> as ErrorType>::Error;
}

impl InputPin for Stm32InputPin<'_> {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        InputPin::is_high(&mut self.0)
    }

    fn is_low(&mut self) -> Result<bool, Self::Error> {
        InputPin::is_low(&mut self.0)
    }
}

impl OutputPin for Stm32OutputPin<'_> {
    fn set_high(&mut self) -> Result<(), Self::Error> {
        OutputPin::set_high(&mut self.0)
    }

    fn set_low(&mut self) -> Result<(), Self::Error> {
        OutputPin::set_low(&mut self.0)
    }
}

impl InputBlock for Stm32InputPin<'_> {
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

impl OutputBlock for Stm32OutputPin<'_> {
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
