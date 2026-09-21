use pictorus_blocks::AdcBlockParams;
use pictorus_internal::protocols::Flush;
use pictorus_traits::{ModelClock, InputBlock, PassBy};

pub struct SimAdc {}

impl SimAdc {
    pub fn new() -> Self {
        SimAdc {}
    }
}

impl Default for SimAdc {
    fn default() -> Self {
        Self::new()
    }
}

impl Flush for SimAdc {
    fn flush(&mut self) {
        // No-op for simulation
    }
}

impl InputBlock for SimAdc {
    type Parameters = AdcBlockParams;
    type Output = u16;

    fn input(
        &mut self,
        _parameters: &Self::Parameters,
        _model_clock: &dyn ModelClock,
    ) -> PassBy<'_, Self::Output> {
        0
    }
}
