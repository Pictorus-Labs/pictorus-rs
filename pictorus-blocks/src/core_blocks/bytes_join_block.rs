extern crate alloc;
use crate::byte_data::parse_string_to_bytes;
use crate::traits::Serialize;
use alloc::vec::Vec;
use block_data::BlockData as OldBlockData;
use pictorus_traits::{ByteSliceSignal, Pass, PassBy, ProcessBlock};

/// Joins multiple signals into a single byte slice by
/// serializing each signal and joining them with a delimiter.
pub struct BytesJoinBlock<T: Apply> {
    pub data: OldBlockData,
    buffer: Vec<u8>,
    _unused: core::marker::PhantomData<T>,
}

/// Parameters for the BytesJoinBlock
pub struct Parameters {
    /// The delimiter to use when joining the signals
    pub delimiter: Vec<u8>,
}

impl Parameters {
    pub fn new(delimiter: &str) -> Self {
        Self {
            delimiter: parse_string_to_bytes(delimiter),
        }
    }
}

impl<T: Apply> Default for BytesJoinBlock<T> {
    fn default() -> Self {
        Self {
            data: OldBlockData::from_bytes(b""),
            buffer: Vec::new(),
            _unused: core::marker::PhantomData,
        }
    }
}

impl<T: Apply> ProcessBlock for BytesJoinBlock<T> {
    type Parameters = Parameters;
    type Inputs = T;
    type Output = ByteSliceSignal;

    fn process(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<Self::Output> {
        self.buffer = T::apply(inputs, parameters);
        self.data.set_bytes(&self.buffer);
        &self.buffer
    }
}

pub trait Apply: Pass {
    fn apply(input: PassBy<Self>, params: &Parameters) -> Vec<u8>;
}

impl<A: Serialize> Apply for A {
    fn apply(input: PassBy<Self>, _params: &Parameters) -> Vec<u8> {
        A::to_bytes_default(input)
    }
}

impl<A: Serialize, B: Serialize> Apply for (A, B) {
    fn apply(input: PassBy<Self>, params: &Parameters) -> Vec<u8> {
        let entries = [A::to_bytes_default(input.0), B::to_bytes_default(input.1)];
        entries.join(params.delimiter.as_slice())
    }
}

impl<A: Serialize, B: Serialize, C: Serialize> Apply for (A, B, C) {
    fn apply(input: PassBy<Self>, params: &Parameters) -> Vec<u8> {
        let entries = [
            A::to_bytes_default(input.0),
            B::to_bytes_default(input.1),
            C::to_bytes_default(input.2),
        ];
        entries.join(params.delimiter.as_slice())
    }
}

impl<A: Serialize, B: Serialize, C: Serialize, D: Serialize> Apply for (A, B, C, D) {
    fn apply(input: PassBy<Self>, params: &Parameters) -> Vec<u8> {
        let entries = [
            A::to_bytes_default(input.0),
            B::to_bytes_default(input.1),
            C::to_bytes_default(input.2),
            D::to_bytes_default(input.3),
        ];
        entries.join(params.delimiter.as_slice())
    }
}

impl<A: Serialize, B: Serialize, C: Serialize, D: Serialize, E: Serialize> Apply
    for (A, B, C, D, E)
{
    fn apply(input: PassBy<Self>, params: &Parameters) -> Vec<u8> {
        let entries = [
            A::to_bytes_default(input.0),
            B::to_bytes_default(input.1),
            C::to_bytes_default(input.2),
            D::to_bytes_default(input.3),
            E::to_bytes_default(input.4),
        ];
        entries.join(params.delimiter.as_slice())
    }
}

impl<A: Serialize, B: Serialize, C: Serialize, D: Serialize, E: Serialize, F: Serialize> Apply
    for (A, B, C, D, E, F)
{
    fn apply(input: PassBy<Self>, params: &Parameters) -> Vec<u8> {
        let entries = [
            A::to_bytes_default(input.0),
            B::to_bytes_default(input.1),
            C::to_bytes_default(input.2),
            D::to_bytes_default(input.3),
            E::to_bytes_default(input.4),
            F::to_bytes_default(input.5),
        ];
        entries.join(params.delimiter.as_slice())
    }
}

impl<
        A: Serialize,
        B: Serialize,
        C: Serialize,
        D: Serialize,
        E: Serialize,
        F: Serialize,
        G: Serialize,
    > Apply for (A, B, C, D, E, F, G)
{
    fn apply(input: PassBy<Self>, params: &Parameters) -> Vec<u8> {
        let entries = [
            A::to_bytes_default(input.0),
            B::to_bytes_default(input.1),
            C::to_bytes_default(input.2),
            D::to_bytes_default(input.3),
            E::to_bytes_default(input.4),
            F::to_bytes_default(input.5),
            G::to_bytes_default(input.6),
        ];
        entries.join(params.delimiter.as_slice())
    }
}

impl<
        A: Serialize,
        B: Serialize,
        C: Serialize,
        D: Serialize,
        E: Serialize,
        F: Serialize,
        G: Serialize,
        H: Serialize,
    > Apply for (A, B, C, D, E, F, G, H)
{
    fn apply(input: PassBy<Self>, params: &Parameters) -> Vec<u8> {
        let entries = [
            A::to_bytes_default(input.0),
            B::to_bytes_default(input.1),
            C::to_bytes_default(input.2),
            D::to_bytes_default(input.3),
            E::to_bytes_default(input.4),
            F::to_bytes_default(input.5),
            G::to_bytes_default(input.6),
            H::to_bytes_default(input.7),
        ];
        entries.join(params.delimiter.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use std::println;

    use super::*;
    use crate::testing::StubContext;
    use alloc::string::ToString;
    use pictorus_traits::Matrix;

    #[test]
    fn test_bytes_join_block() {
        let ctxt = StubContext::default();
        let params = Parameters::new("/ ");
        let mut block = BytesJoinBlock::<(f64, Matrix<2, 3, f64>, ByteSliceSignal)>::default();

        // First input to the block is a scalar
        let signal1 = 1.0;

        // Second input is a matrix
        // Our matrices our col-major, so we would expect an input matrix of
        // [[1.0, 4.0], [2.0, 5.0], [3.0, 6.0]] to be serialized as
        // [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]
        let signal2 = Matrix {
            data: [[1.0, 4.0], [2.0, 5.0], [3.0, 6.0]],
        };

        // Third input is some bytes data
        let signal3 = b"hello there";
        let res = block.process(
            &params,
            &ctxt,
            (signal1.as_by(), signal2.as_by(), signal3.as_by()),
        );
        println!("{}", std::str::from_utf8(res).unwrap());

        let expected_string = "1.0/ [[1.0,2.0,3.0],[4.0,5.0,6.0]]/ hello there".to_string();
        assert_eq!(res, expected_string.as_bytes());
        assert_eq!(block.data.raw_string(), expected_string);
    }

    #[test]
    fn test_bytes_join_block_non_ascii_input() {
        let ctxt = StubContext::default();
        let params = Parameters::new("⚡");
        let mut block = BytesJoinBlock::<(ByteSliceSignal, ByteSliceSignal)>::default();

        let signal1 = "привет".as_bytes();
        let signal2 = "こんにちは".as_bytes();
        let res = block.process(&params, &ctxt, (signal1, signal2));

        let expected_string = "привет⚡こんにちは".to_string();
        assert_eq!(res, expected_string.as_bytes());
        assert_eq!(block.data.raw_string(), expected_string);
    }

    #[test]
    fn test_bytes_join_non_utf8_input() {
        let ctxt = StubContext::default();
        let params = Parameters::new(r"\x99");
        let mut block = BytesJoinBlock::<(ByteSliceSignal, ByteSliceSignal)>::default();

        let signal1 = b"\x80\x81\x82\x83";
        let signal2 = b"\x84\x85\x86\x87";
        let res = block.process(&params, &ctxt, (signal1, signal2));

        let expected = b"\x80\x81\x82\x83\x99\x84\x85\x86\x87";
        assert_eq!(res, expected);
        assert_eq!(block.data.to_bytes().as_slice(), expected);
    }
}
