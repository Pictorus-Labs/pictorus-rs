use block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{Context, Matrix, Pass, PassBy, ProcessBlock};

use crate::traits::Float;

/// Parameters for Dac Block
pub struct Parameters {}

impl Default for Parameters {
    fn default() -> Self {
        Self::new()
    }
}

impl Parameters {
    pub fn new() -> Self {
        Self {}
    }
}

/// Dac Block buffers data and passes it on to the hardware protocol that implements the
/// `OutputBlock` trait.
pub struct DacBlock<O: Pass> {
    pub data: OldBlockData,
    buffer: Option<O>,
}

impl<O> Default for DacBlock<O>
where
    O: Pass + Default,
    OldBlockData: FromPass<O>,
{
    fn default() -> Self {
        DacBlock {
            data: <OldBlockData as FromPass<O>>::from_pass(<O>::default().as_by()),
            buffer: None,
        }
    }
}

impl<F> ProcessBlock for DacBlock<Matrix<1, 2, F>>
where
    F: Float,
    OldBlockData: FromPass<Matrix<1, 2, F>>,
{
    type Parameters = Parameters;
    type Inputs = Matrix<1, 2, F>;
    type Output = Matrix<1, 2, F>;

    fn process<'b>(
        &'b mut self,
        _parameters: &Self::Parameters,
        _context: &dyn Context,
        input: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        let output = self.buffer.insert(*input);
        self.data = OldBlockData::from_pass(output);
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;

    #[test]
    fn test_dac_block() {
        let mut dac_block = DacBlock::<Matrix<1, 2, f64>>::default();
        let context = StubContext::default();
        let output =
            dac_block.process(&Parameters::new(), &context, &Matrix { data: [[1.], [2.]] });
        assert_eq!(output.data, [[1.], [2.]]);
    }
}
