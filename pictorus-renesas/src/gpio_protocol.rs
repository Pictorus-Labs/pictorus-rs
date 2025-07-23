use embedded_hal::digital::{InputPin, OutputPin};
use pictorus_traits::{InputBlock, OutputBlock};
use ra4m2_hal::gpio::{Input, Output, PullDown, PushPull};

pub struct RenesasInputPin<const N: u8>(ra4m2_hal::gpio::port4::Pin<Input<PullDown>, N>);

impl<const N: u8> RenesasInputPin<N> {
    pub fn new(inner: ra4m2_hal::gpio::port4::Pin<Input<PullDown>, N>) -> Self {
        RenesasInputPin(inner)
    }
}

impl<const N: u8> InputBlock for RenesasInputPin<N> {
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

pub struct RenesasOutputPin<const N: u8>(ra4m2_hal::gpio::port4::Pin<Output<PushPull>, N>);

impl<const N: u8> RenesasOutputPin<N> {
    pub fn new(inner: ra4m2_hal::gpio::port4::Pin<Output<PushPull>, N>) -> Self {
        RenesasOutputPin(inner)
    }
}

impl<const N: u8> OutputBlock for RenesasOutputPin<N> {
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
