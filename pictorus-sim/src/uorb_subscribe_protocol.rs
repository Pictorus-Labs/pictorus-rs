use std::convert::Infallible;

use pictorus_traits::{Context, InputBlock, PassBy};

pub struct SimUorbSubscribeProtocolParameters {}

pub struct SimUorbSubscribeProtocol {}

impl SimUorbSubscribeProtocol {
    pub fn new() -> Result<Self, Infallible> {
        Ok(SimUorbSubscribeProtocol {})
    }
}

impl InputBlock for SimUorbSubscribeProtocol {
    type Parameters = SimUorbSubscribeProtocolParameters;
    type Output = f64;

    fn input(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn Context,
    ) -> PassBy<'_, Self::Output> {
        0.0
    }
}
