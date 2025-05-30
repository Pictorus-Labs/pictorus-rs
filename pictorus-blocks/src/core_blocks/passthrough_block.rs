// TODO: Currently we require alloc in this crate to support OldBlockData,
// but eventually this crate should be no_std and no_alloc. When we remove
// OldBlockData, we should also update this block to function without alloc
extern crate alloc;

use crate::traits::DefaultStorage;
use pictorus_block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{PassBy, ProcessBlock};

// A block that passes through the input data, storing it in a buffer.
//
// Eventually it would be better to remove this block and just use the input value directly,
// but we need to maintain it for now to keep the old block data system working.
#[doc(hidden)]
pub struct PassthroughBlock<T: DefaultStorage>
where
    OldBlockData: FromPass<T>,
{
    pub data: OldBlockData,
    buffer: T::Storage,
}

impl<T: DefaultStorage> Default for PassthroughBlock<T>
where
    OldBlockData: FromPass<T>,
{
    fn default() -> Self {
        Self {
            data: <OldBlockData as FromPass<T>>::from_pass(<T as DefaultStorage>::from_storage(
                &T::default_storage(),
            )),
            buffer: T::default_storage(),
        }
    }
}

#[doc(hidden)]
#[derive(Default)]
pub struct Parameters;

impl Parameters {
    pub fn new() -> Parameters {
        Parameters {}
    }
}

impl<T: DefaultStorage> ProcessBlock for PassthroughBlock<T>
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
        T::copy_into(input, &mut self.buffer);
        let res = <T as DefaultStorage>::from_storage(&self.buffer);
        self.data = <OldBlockData as FromPass<T>>::from_pass(res);
        res
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::StubContext;
    use pictorus_traits::{ByteSliceSignal, Matrix, Pass};

    use super::*;

    #[test]
    fn test_passthrough_block_scalar() {
        let ctxt = StubContext::default();
        let params = Parameters;
        let mut block = PassthroughBlock::<f64>::default();

        let input = 99.999;
        let output = block.process(&params, &ctxt, input.as_by());
        assert_eq!(output, input);
        assert_eq!(block.data.scalar(), input);
    }

    #[test]
    fn test_passthrough_block_bytes() {
        let ctxt = StubContext::default();
        let params = Parameters;
        let mut block = PassthroughBlock::<ByteSliceSignal>::default();

        let input = b"hello world";
        let output = block.process(&params, &ctxt, input.as_slice());
        assert_eq!(output, input);
        assert_eq!(block.data.to_raw_bytes(), input);

        let input = b"";
        let output = block.process(&params, &ctxt, input.as_slice());
        assert_eq!(output, input);
        assert_eq!(block.data.to_raw_bytes(), input);
    }

    #[test]
    fn test_passthrough_block_matrix() {
        let ctxt = StubContext::default();
        let params = Parameters;
        let mut block = PassthroughBlock::<Matrix<2, 2, f64>>::default();

        let input = Matrix {
            data: [[1.0, 2.0], [3.0, 4.0]],
        };
        let output = block.process(&params, &ctxt, input.as_by());
        assert_eq!(output, &input);
        assert_eq!(block.data.get_data().as_slice(), input.data.as_flattened());
    }
}
