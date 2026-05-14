use pictorus_block_data::BlockData;
use pictorus_traits::{ByteSliceSignal, GeneratorBlock};

pub struct Parameters<const CHARS: usize> {
    pub value: [u8; CHARS],
}

impl<const CHARS: usize> Parameters<CHARS> {
    pub fn new(input: [u8; CHARS]) -> Self {
        Self { value: input }
    }
}

/// Output a constant byte slice as a signal.
pub struct BytesLiteralBlock<const CHARS: usize> {
    buffer: [u8; CHARS],
    pub data: BlockData,
}

impl<const CHARS: usize> Default for BytesLiteralBlock<CHARS> {
    fn default() -> Self {
        Self {
            data: BlockData::from_bytes(&[0; CHARS]),
            buffer: [0; CHARS],
        }
    }
}

impl<const CHARS: usize> GeneratorBlock for BytesLiteralBlock<CHARS> {
    type Output = ByteSliceSignal;
    type Parameters = Parameters<CHARS>;

    fn generate(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
    ) -> pictorus_traits::PassBy<'_, Self::Output> {
        self.data = BlockData::from_bytes(&parameters.value);
        self.buffer = parameters.value;
        &self.buffer
    }

    fn buffer(&self) -> pictorus_traits::PassBy<'_, Self::Output> {
        &self.buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::testing::StubContext;

    #[test]
    fn test_bytes_literal_default_buffer_no_panic() {
        let block = BytesLiteralBlock::<11>::default();
        assert_eq!(block.buffer(), &[0u8; 11]);
    }

    #[test]
    fn test_constant_block() {
        let mut block = BytesLiteralBlock::<11>::default();

        let bytes_literal_ic = *b"Hello World";

        let parameters = Parameters::new(bytes_literal_ic);
        let context = StubContext::default();

        let output = block.generate(&parameters, &context).to_vec();
        assert_eq!(output, "Hello World".as_bytes());
        assert_eq!(block.data, BlockData::from_bytes("Hello World".as_bytes()));
        assert_eq!(block.buffer(), output.as_slice());
    }
}
