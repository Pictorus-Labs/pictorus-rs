use embedded_hal::digital::{InputPin, OutputPin};
use pictorus_traits::{InputBlock, OutputBlock};

pub struct RenesasInputPin<P: InputPin>(P);

impl<P: InputPin> RenesasInputPin<P> {
    pub fn new(inner: P) -> Self {
        RenesasInputPin(inner)
    }
}

impl<P: InputPin> InputBlock for RenesasInputPin<P> {
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

pub struct RenesasOutputPin<P: OutputPin>(P);

impl<P: OutputPin> RenesasOutputPin<P> {
    pub fn new(inner: P) -> Self {
        RenesasOutputPin(inner)
    }
}

impl<P: OutputPin> OutputBlock for RenesasOutputPin<P> {
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
