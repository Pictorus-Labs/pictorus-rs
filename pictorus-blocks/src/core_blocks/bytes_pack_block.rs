extern crate alloc;
use crate::byte_data::{parse_byte_data_spec, try_pack_data, ByteOrderSpec, DataType};
use crate::traits::Scalar;
use alloc::vec::Vec;
use pictorus_block_data::BlockData as OldBlockData;
use pictorus_traits::{ByteSliceSignal, Pass, PassBy, ProcessBlock};

/// Packs scalar inputs into a byte buffer according to the provided data spec.
pub struct BytesPackBlock<T: Apply> {
    pub data: OldBlockData,
    buffer: Vec<u8>,
    _unused: core::marker::PhantomData<T>,
}

impl<T: Apply> Default for BytesPackBlock<T> {
    fn default() -> Self {
        Self {
            data: OldBlockData::from_bytes(b""),
            buffer: Vec::new(),
            _unused: core::marker::PhantomData,
        }
    }
}

impl<T: Apply> ProcessBlock for BytesPackBlock<T> {
    type Inputs = T;
    type Output = ByteSliceSignal;
    type Parameters = T::Params;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        self.buffer = T::pack_bytes(inputs, parameters);
        self.data = OldBlockData::from_bytes(&self.buffer);
        self.buffer.as_slice()
    }
}

/// Each input must be assigned a data spec that describes how to pack the input into bytes.
/// The data spec consists of a data type and a byte order (e.g. (f32:BigEndian))
pub struct Parameters<const N: usize> {
    pub pack_spec: [(DataType, ByteOrderSpec); N],
}

impl<const N: usize> Parameters<N> {
    /// This constructor takes a slice of strings that represent the data spec for each input.
    pub fn new<S: AsRef<str>>(pack_spec_str: &[S]) -> Self {
        let pack_spec = parse_byte_data_spec(pack_spec_str)
            .try_into()
            .expect("Bytes Data Spec is incorrectly sized for the number of inputs");
        Self { pack_spec }
    }
}

pub trait AppendBytes: Scalar {
    fn append_bytes(&self, data_spec: (DataType, ByteOrderSpec), buffer: &mut Vec<u8>) -> usize;
}

impl AppendBytes for f64 {
    fn append_bytes(&self, data_spec: (DataType, ByteOrderSpec), buffer: &mut Vec<u8>) -> usize {
        let mut scratch = [0u8; 16]; // 16 bytes is the size of i128 which is the largest output spec we support
        let n = match data_spec.1 {
            ByteOrderSpec::BigEndian => {
                try_pack_data::<byteorder::BigEndian>(&mut scratch, *self, data_spec.0)
            }
            ByteOrderSpec::LittleEndian => {
                try_pack_data::<byteorder::LittleEndian>(&mut scratch, *self, data_spec.0)
            }
        }
        .expect("Scratch should always be big enough, which is the only way to produce an error");
        buffer.extend_from_slice(scratch[..n].as_ref());
        n
    }
}

pub trait Apply: Pass {
    type Params;
    fn pack_bytes(input: PassBy<Self>, params: &Self::Params) -> Vec<u8>;
}

impl<S: AppendBytes> Apply for S {
    type Params = Parameters<1>;
    fn pack_bytes(input: PassBy<Self>, params: &Self::Params) -> Vec<u8> {
        let mut buffer = Vec::new();
        input.append_bytes(params.pack_spec[0], &mut buffer);
        buffer
    }
}

impl<S1: AppendBytes, S2: AppendBytes> Apply for (S1, S2) {
    type Params = Parameters<2>;
    fn pack_bytes(input: PassBy<Self>, params: &Self::Params) -> Vec<u8> {
        let mut buffer = Vec::new();
        seq_macro::seq!(N in 0..2 {
            input.N.append_bytes(params.pack_spec[N], &mut buffer);
        });
        buffer
    }
}

impl<S1: AppendBytes, S2: AppendBytes, S3: AppendBytes> Apply for (S1, S2, S3) {
    type Params = Parameters<3>;
    fn pack_bytes(input: PassBy<Self>, params: &Self::Params) -> Vec<u8> {
        let mut buffer = Vec::new();
        seq_macro::seq!(N in 0..3 {
            input.N.append_bytes(params.pack_spec[N], &mut buffer);
        });
        buffer
    }
}

impl<S1: AppendBytes, S2: AppendBytes, S3: AppendBytes, S4: AppendBytes> Apply
    for (S1, S2, S3, S4)
{
    type Params = Parameters<4>;
    fn pack_bytes(input: PassBy<Self>, params: &Self::Params) -> Vec<u8> {
        let mut buffer = Vec::new();
        seq_macro::seq!(N in 0..4 {
            input.N.append_bytes(params.pack_spec[N], &mut buffer);
        });
        buffer
    }
}

impl<S1: AppendBytes, S2: AppendBytes, S3: AppendBytes, S4: AppendBytes, S5: AppendBytes> Apply
    for (S1, S2, S3, S4, S5)
{
    type Params = Parameters<5>;
    fn pack_bytes(input: PassBy<Self>, params: &Self::Params) -> Vec<u8> {
        let mut buffer = Vec::new();
        seq_macro::seq!(N in 0..5 {
            input.N.append_bytes(params.pack_spec[N], &mut buffer);
        });
        buffer
    }
}

impl<
        S1: AppendBytes,
        S2: AppendBytes,
        S3: AppendBytes,
        S4: AppendBytes,
        S5: AppendBytes,
        S6: AppendBytes,
    > Apply for (S1, S2, S3, S4, S5, S6)
{
    type Params = Parameters<6>;
    fn pack_bytes(input: PassBy<Self>, params: &Self::Params) -> Vec<u8> {
        let mut buffer = Vec::new();
        seq_macro::seq!(N in 0..6 {
            input.N.append_bytes(params.pack_spec[N], &mut buffer);
        });
        buffer
    }
}

impl<
        S1: AppendBytes,
        S2: AppendBytes,
        S3: AppendBytes,
        S4: AppendBytes,
        S5: AppendBytes,
        S6: AppendBytes,
        S7: AppendBytes,
    > Apply for (S1, S2, S3, S4, S5, S6, S7)
{
    type Params = Parameters<7>;
    fn pack_bytes(input: PassBy<Self>, params: &Self::Params) -> Vec<u8> {
        let mut buffer = Vec::new();
        seq_macro::seq!(N in 0..7 {
            input.N.append_bytes(params.pack_spec[N], &mut buffer);
        });
        buffer
    }
}

impl<
        S1: AppendBytes,
        S2: AppendBytes,
        S3: AppendBytes,
        S4: AppendBytes,
        S5: AppendBytes,
        S6: AppendBytes,
        S7: AppendBytes,
        S8: AppendBytes,
    > Apply for (S1, S2, S3, S4, S5, S6, S7, S8)
{
    type Params = Parameters<8>;
    fn pack_bytes(input: PassBy<Self>, params: &Self::Params) -> Vec<u8> {
        let mut buffer = Vec::new();
        seq_macro::seq!(N in 0..8 {
            input.N.append_bytes(params.pack_spec[N], &mut buffer);
        });
        buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use byteorder::WriteBytesExt;

    #[test]
    fn test_bytes_pack_block_1_input() {
        let context = StubContext::default();
        let params = Parameters::new(&["I8:BigEndian"]);
        let mut block = BytesPackBlock::<f64>::default();
        let inputs = 255.0;

        let expected = {
            let mut expected = Vec::new();
            expected.write_i8(inputs as i8).unwrap();
            expected
        };

        let output = block.process(&params, &context, inputs);
        assert_eq!(output, expected.as_slice());
        assert_eq!(block.data, OldBlockData::from_bytes(&expected));
    }

    #[test]
    fn test_bytes_pack_block_2_inputs() {
        let context = StubContext::default();
        let params = Parameters::new(&["F32:BigEndian", "U24:LittleEndian"]);
        let mut block = BytesPackBlock::<(f64, f64)>::default();
        let inputs = (255.0, 123.0);

        let expected = {
            let mut expected = Vec::new();
            expected
                .write_f32::<byteorder::BigEndian>(inputs.0 as f32)
                .unwrap();
            expected
                .write_u24::<byteorder::LittleEndian>(inputs.1 as u32)
                .unwrap();
            expected
        };

        let output = block.process(&params, &context, inputs);
        assert_eq!(output, expected.as_slice());
        assert_eq!(block.data, OldBlockData::from_bytes(&expected));

        // Make sure buffer is cleared between runs, this logic is just in one test because it is written
        // for the ProcessBlock impl and so is shared between all input possibilities
        let inputs = (42.0, 1337.0);
        let expected = {
            let mut expected = Vec::new();
            expected
                .write_f32::<byteorder::BigEndian>(inputs.0 as f32)
                .unwrap();
            expected
                .write_u24::<byteorder::LittleEndian>(inputs.1 as u32)
                .unwrap();
            expected
        };
        let output = block.process(&params, &context, inputs);
        assert_eq!(output, expected.as_slice());
        assert_eq!(block.data, OldBlockData::from_bytes(&expected));
    }

    #[test]
    fn test_bytes_pack_block_3_inputs() {
        let context = StubContext::default();
        let params = Parameters::new(&["I16:BigEndian", "U16:LittleEndian", "I32:BigEndian"]);
        let mut block = BytesPackBlock::<(f64, f64, f64)>::default();
        let inputs = (1000.0, 12345.0, -1234.0);

        let expected = {
            let mut expected = Vec::new();
            expected
                .write_i16::<byteorder::BigEndian>(inputs.0 as i16)
                .unwrap();
            expected
                .write_u16::<byteorder::LittleEndian>(inputs.1 as u16)
                .unwrap();
            expected
                .write_i32::<byteorder::BigEndian>(inputs.2 as i32)
                .unwrap();
            expected
        };

        let output = block.process(&params, &context, inputs);
        assert_eq!(output, expected.as_slice());
        assert_eq!(block.data, OldBlockData::from_bytes(&expected));
    }

    #[test]
    fn test_bytes_pack_block_4_inputs() {
        let context = StubContext::default();
        let params = Parameters::new(&[
            "I16:BigEndian",
            "U16:LittleEndian",
            "I32:BigEndian",
            "F64:LittleEndian",
        ]);
        let mut block = BytesPackBlock::<(f64, f64, f64, f64)>::default();
        let inputs = (1000.0, 12345.0, -1234.0, 3.1);

        let expected = {
            let mut expected = Vec::new();
            expected
                .write_i16::<byteorder::BigEndian>(inputs.0 as i16)
                .unwrap();
            expected
                .write_u16::<byteorder::LittleEndian>(inputs.1 as u16)
                .unwrap();
            expected
                .write_i32::<byteorder::BigEndian>(inputs.2 as i32)
                .unwrap();
            expected
                .write_f64::<byteorder::LittleEndian>(inputs.3)
                .unwrap();
            expected
        };

        let output = block.process(&params, &context, inputs);
        assert_eq!(output, expected.as_slice());
        assert_eq!(block.data, OldBlockData::from_bytes(&expected));
    }

    #[test]
    fn test_bytes_pack_block_5_inputs() {
        let context = StubContext::default();
        let params = Parameters::new(&[
            "I16:BigEndian",
            "U16:LittleEndian",
            "I32:BigEndian",
            "F64:LittleEndian",
            "U8:BigEndian",
        ]);
        let mut block = BytesPackBlock::<(f64, f64, f64, f64, f64)>::default();
        let inputs = (1000.0, 12345.0, -1234.0, 3.1, 255.0);

        let expected = {
            let mut expected = Vec::new();
            expected
                .write_i16::<byteorder::BigEndian>(inputs.0 as i16)
                .unwrap();
            expected
                .write_u16::<byteorder::LittleEndian>(inputs.1 as u16)
                .unwrap();
            expected
                .write_i32::<byteorder::BigEndian>(inputs.2 as i32)
                .unwrap();
            expected
                .write_f64::<byteorder::LittleEndian>(inputs.3)
                .unwrap();
            expected.write_u8(inputs.4 as u8).unwrap();
            expected
        };

        let output = block.process(&params, &context, inputs);
        assert_eq!(output, expected.as_slice());
        assert_eq!(block.data, OldBlockData::from_bytes(&expected));
    }

    #[test]
    fn test_bytes_pack_block_6_inputs() {
        let context = StubContext::default();
        let params = Parameters::new(&[
            "I16:BigEndian",
            "U16:LittleEndian",
            "I32:BigEndian",
            "F64:LittleEndian",
            "U8:BigEndian",
            "I64:LittleEndian",
        ]);
        let mut block = BytesPackBlock::<(f64, f64, f64, f64, f64, f64)>::default();
        let inputs = (1000.0, 12345.0, -1234.0, 3.1, 255.0, -1234567890.0);

        let expected = {
            let mut expected = Vec::new();
            expected
                .write_i16::<byteorder::BigEndian>(inputs.0 as i16)
                .unwrap();
            expected
                .write_u16::<byteorder::LittleEndian>(inputs.1 as u16)
                .unwrap();
            expected
                .write_i32::<byteorder::BigEndian>(inputs.2 as i32)
                .unwrap();
            expected
                .write_f64::<byteorder::LittleEndian>(inputs.3)
                .unwrap();
            expected.write_u8(inputs.4 as u8).unwrap();
            expected
                .write_i64::<byteorder::LittleEndian>(inputs.5 as i64)
                .unwrap();
            expected
        };

        let output = block.process(&params, &context, inputs);
        assert_eq!(output, expected.as_slice());
        assert_eq!(block.data, OldBlockData::from_bytes(&expected));
    }

    #[test]
    fn test_bytes_pack_block_7_inputs() {
        let context = StubContext::default();
        let params = Parameters::new(&[
            "I16:BigEndian",
            "U16:LittleEndian",
            "I32:BigEndian",
            "F64:LittleEndian",
            "U8:BigEndian",
            "I64:LittleEndian",
            "F32:BigEndian",
        ]);
        let mut block = BytesPackBlock::<(f64, f64, f64, f64, f64, f64, f64)>::default();
        let inputs = (1000.0, 12345.0, -1234.0, 3.1, 255.0, -1234567890.0, 1.0);

        let expected = {
            let mut expected = Vec::new();
            expected
                .write_i16::<byteorder::BigEndian>(inputs.0 as i16)
                .unwrap();
            expected
                .write_u16::<byteorder::LittleEndian>(inputs.1 as u16)
                .unwrap();
            expected
                .write_i32::<byteorder::BigEndian>(inputs.2 as i32)
                .unwrap();
            expected
                .write_f64::<byteorder::LittleEndian>(inputs.3)
                .unwrap();
            expected.write_u8(inputs.4 as u8).unwrap();
            expected
                .write_i64::<byteorder::LittleEndian>(inputs.5 as i64)
                .unwrap();
            expected
                .write_f32::<byteorder::BigEndian>(inputs.6 as f32)
                .unwrap();
            expected
        };

        let output = block.process(&params, &context, inputs);
        assert_eq!(output, expected.as_slice());
        assert_eq!(block.data, OldBlockData::from_bytes(&expected));
    }

    #[test]
    fn test_bytes_pack_block_8_inputs() {
        let context = StubContext::default();
        let params = Parameters::new(&[
            "I16:BigEndian",
            "U16:LittleEndian",
            "I32:BigEndian",
            "F64:LittleEndian",
            "U8:BigEndian",
            "I64:LittleEndian",
            "F32:BigEndian",
            "U32:LittleEndian",
        ]);
        let mut block = BytesPackBlock::<(f64, f64, f64, f64, f64, f64, f64, f64)>::default();
        let inputs = (
            1000.0,
            12345.0,
            -1234.0,
            3.1,
            255.0,
            -1234567890.0,
            1.0,
            4294967295.0,
        );

        let expected = {
            let mut expected = Vec::new();
            expected
                .write_i16::<byteorder::BigEndian>(inputs.0 as i16)
                .unwrap();
            expected
                .write_u16::<byteorder::LittleEndian>(inputs.1 as u16)
                .unwrap();
            expected
                .write_i32::<byteorder::BigEndian>(inputs.2 as i32)
                .unwrap();
            expected
                .write_f64::<byteorder::LittleEndian>(inputs.3)
                .unwrap();
            expected.write_u8(inputs.4 as u8).unwrap();
            expected
                .write_i64::<byteorder::LittleEndian>(inputs.5 as i64)
                .unwrap();
            expected
                .write_f32::<byteorder::BigEndian>(inputs.6 as f32)
                .unwrap();
            expected
                .write_u32::<byteorder::LittleEndian>(inputs.7 as u32)
                .unwrap();
            expected
        };

        let output = block.process(&params, &context, inputs);
        assert_eq!(output, expected.as_slice());
        assert_eq!(block.data, OldBlockData::from_bytes(&expected));
    }
}
