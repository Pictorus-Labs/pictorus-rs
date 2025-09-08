use std::convert::Infallible;

use pictorus_traits::OutputBlock;

pub struct SimUorbAdvertiseProtocolParameters {}

pub struct SimUorbAdvertiseProtocol {}

impl SimUorbAdvertiseProtocol {
    pub fn new() -> Result<Self, Infallible> {
        Ok(SimUorbAdvertiseProtocol {})
    }
}

impl OutputBlock for SimUorbAdvertiseProtocol {
    type Parameters = SimUorbAdvertiseProtocolParameters;
    type Inputs = f64;

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        _inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) {
    }
}
