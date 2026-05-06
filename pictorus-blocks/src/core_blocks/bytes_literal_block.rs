use generic_array::{ArrayLength, GenericArray};
use pictorus_block_data::BlockData;
use pictorus_traits::{ByteSliceSignal, GeneratorBlock};
use typenum::{Const, NonZero, Sub1, ToUInt, B1, U};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Parameters<N: ArrayLength> {
    pub value: GenericArray<u8, N>,
}

impl<const CHARS: usize> Parameters<Const<CHARS>>
where
    Const<CHARS>: ArrayLength,
{
    pub fn new(input: [u8; CHARS]) -> Self {
        Self {
            value: GenericArray::from(input),
        }
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

impl<const CHARS: usize> GeneratorBlock for BytesLiteralBlock<CHARS>
where
    Const<CHARS>: ArrayLength,
{
    type Output = ByteSliceSignal;
    type Parameters = Parameters<Const<CHARS>>;

    fn generate(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
    ) -> pictorus_traits::PassBy<'_, Self::Output> {
        self.data = BlockData::from_bytes(&parameters.value);
        self.buffer.copy_from_slice(&parameters.value);
        &self.buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::testing::StubContext;
    use pictorus_block_data::ToPass;
    use std::string::String;

    #[test]
    fn test_constant_block() {
        let mut block = BytesLiteralBlock::<11>::default();

        let bytes_literal_ic = BlockData::from_bytes(String::from("Hello World").as_bytes());

        let parameters = Parameters::new(bytes_literal_ic.to_pass());
        let context = StubContext::default();

        let output = block.generate(&parameters, &context);
        assert_eq!(output, "Hello World".as_bytes());
        assert_eq!(block.data, BlockData::from_bytes("Hello World".as_bytes()));
    }
}
