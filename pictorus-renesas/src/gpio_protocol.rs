use embedded_hal_02::digital::v2::{InputPin, OutputPin};
use pictorus_traits::{InputBlock, OutputBlock};
use ra4m2_hal::gpio::{Input, Output, PullDown, PushPull};

pub struct RenesasInputPin(ra4m2_hal::gpio::port4::Pin<Input<PullDown>>);

impl RenesasInputPin {
    pub fn new(inner: ra4m2_hal::gpio::port4::Pin<Input<PullDown>>) -> Self {
        RenesasInputPin(inner)
    }
}

impl InputBlock for RenesasInputPin {
    type Output = f64;
    type Parameters = pictorus_blocks::GpioInputBlockParams;

    fn input(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
    ) -> pictorus_traits::PassBy<'_, Self::Output> {
        self.0.is_high().unwrap_or(false).into()
    }
}

pub struct RenesasOutputPin(ra4m2_hal::gpio::port4::Pin<Output<PushPull>>);

impl RenesasOutputPin {
    pub fn new(inner: ra4m2_hal::gpio::port4::Pin<Output<PushPull>>) -> Self {
        RenesasOutputPin(inner)
    }
}

impl OutputBlock for RenesasOutputPin {
    type Inputs = bool;
    type Parameters = pictorus_blocks::GpioOutputBlockParams;

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) {
        if inputs {
            self.0.set_high().ok();
        } else {
            self.0.set_low().ok();
        }
    }
}
