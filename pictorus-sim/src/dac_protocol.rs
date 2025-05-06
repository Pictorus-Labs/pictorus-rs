use pictorus_blocks::DacBlockParams;
use pictorus_traits::{Matrix, OutputBlock};

pub struct SimDac {}

impl SimDac {
    pub fn new() -> Self {
        SimDac {}
    }
}

impl Default for SimDac {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputBlock for SimDac {
    type Parameters = DacBlockParams;
    type Inputs = Matrix<1, 2, f64>;

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        _inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) {
        // Do nothing
    }
}
