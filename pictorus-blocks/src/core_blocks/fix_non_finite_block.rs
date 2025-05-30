use crate::traits::Float;
use block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{Pass, PassBy, ProcessBlock};

// A block that ensures all data passed into it is finite, replacing non-finite values with zero.
//
// This block is needed to support legacy behavior where we fix values passed in by user-defined functions
// i.e. EquationBlock and RustBlock. It's unclear if we want to support this behavior in the future.
#[doc(hidden)]
pub struct FixNonFiniteBlock<T: Pass>
where
    OldBlockData: FromPass<T>,
{
    pub data: OldBlockData,
    buffer: T,
}

impl<T: Float> Default for FixNonFiniteBlock<T>
where
    OldBlockData: FromPass<T>,
{
    fn default() -> Self {
        FixNonFiniteBlock {
            data: OldBlockData::from_pass(T::default().as_by()),
            buffer: T::default(),
        }
    }
}

#[derive(Default)]
pub struct Parameters;

impl Parameters {
    pub fn new() -> Parameters {
        Parameters {}
    }
}

impl<T: Float> ProcessBlock for FixNonFiniteBlock<T>
where
    OldBlockData: FromPass<T>,
{
    type Parameters = Parameters;
    type Inputs = T;
    type Output = T;

    fn process<'b>(
        &'b mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        input: PassBy<'_, Self::Inputs>,
    ) -> pictorus_traits::PassBy<'b, Self::Output> {
        let res = if !input.is_finite() {
            T::default()
        } else {
            input
        };
        self.buffer = res;
        self.data = OldBlockData::from_pass(res);
        res
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::StubContext;

    use super::*;

    #[test]
    fn test_passthrough_block_scalar() {
        let ctxt = StubContext::default();
        let params = Parameters;
        let mut block = FixNonFiniteBlock::<f64>::default();

        let input = 99.999;
        let output = block.process(&params, &ctxt, input.as_by());
        assert_eq!(output, input);
        assert_eq!(block.data.scalar(), input);

        let input = f64::NAN;
        let output = block.process(&params, &ctxt, input.as_by());
        assert_eq!(output, 0.0);
        assert_eq!(block.data.scalar(), 0.0);
    }
}
