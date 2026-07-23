use core::time::Duration;

use crate::{
    stale_tracker::{duration_from_ms_f64, StaleTracker},
    traits::Float,
};
use generic_array::{ArrayLength, GenericArray};
use typenum::{Const, NonZero, Sub1, ToUInt, B1, U};

use crate::byte_data::{parse_byte_data_spec, try_unpack_data, ByteOrderSpec, DataType};
use pictorus_traits::{ByteSliceSignal, PassBy, ProcessBlock};

/// Unpacks a byte slice into a specified number of outputs based on the provided data types and byte order.
///
/// The output is `[O; N]` (`O` defaults for `f64` unless otherwise set) where the last element is a `1.0`/`0.0`
/// validity flag — true while the most recent successful unpack is still within the configured `stale_age`.
pub struct BytesUnpackBlock<const N: usize, O: Unpack = f64> {
    buffer: [O; N],
    stale_check: StaleTracker,
}

impl<const N: usize, O: Unpack> Default for BytesUnpackBlock<N, O> {
    fn default() -> Self {
        BytesUnpackBlock {
            buffer: [O::zero(); N],
            stale_check: StaleTracker::default(),
        }
    }
}

pub struct Parameters<N: ArrayLength> {
    // TODO: I think we can use a [(DataType, ByteOrderSpec); N] once we ditch vectors
    // for certain parameters. This would remove a dependency on generic-array (which uses alloc).
    pub pack_spec: GenericArray<(DataType, ByteOrderSpec), N>,
    pub stale_age: Duration,
}

impl<const N: usize, O: Unpack> ProcessBlock for BytesUnpackBlock<N, O>
where
    // To be able to have parameters be of size N-1 we use typenum to express that at the type level
    Const<N>: ToUInt,                   // Ensure N can be converted to a typenum UInt
    U<N>: core::ops::Sub<B1> + NonZero, // Ensure N-1 is non-zero and N-1 is valid
    Sub1<U<N>>: ArrayLength, // Ensure we can use N-1 as an ArrayLength (this is a trait defined by generic-array). `Sub1<U<N>>` is shorthand for `<U<N> as Sub<B1>>::Output`
{
    type Parameters = Parameters<Sub1<U<N>>>;

    type Inputs = ByteSliceSignal;
    type Output = [O; N];

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        let mut new_buffer = [O::zero(); N];
        // Use Unpack::unpack N-1 times to fill the buffer; if it fails at any point,
        // keep the old buffer values. The data at N-1 is the `is_valid` flag.
        let mut inputs = inputs;
        let mut unpack_success = true;
        for (i, elem) in new_buffer.iter_mut().enumerate().take(N - 1) {
            let (val, advanced_data) =
                O::unpack(inputs, parameters.pack_spec[i].0, parameters.pack_spec[i].1);
            if let Some(val) = val {
                *elem = val;
            } else {
                unpack_success = false;
                break;
            }
            inputs = advanced_data;
        }
        if unpack_success {
            self.buffer = new_buffer;
            self.stale_check.mark_updated(context.time());
        }
        self.buffer[N - 1] = if self
            .stale_check
            .is_valid(context.time(), parameters.stale_age)
        {
            O::one()
        } else {
            O::zero()
        };
        &self.buffer
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        &self.buffer
    }
}

impl<N: ArrayLength> Parameters<N> {
    /// This constructor takes a slice of strings that represent the data spec for each input.
    pub fn new<S: AsRef<str>>(pack_spec_str: &[S], stale_age_ms: f64) -> Self {
        let pack_spec = parse_byte_data_spec(pack_spec_str)
            .try_into()
            .expect("Bytes Data Spec is incorrectly sized for the number of inputs");
        Self {
            pack_spec,
            stale_age: duration_from_ms_f64(stale_age_ms),
        }
    }
}

pub trait Unpack: Float {
    fn unpack(data: &[u8], data_type: DataType, byte_order: ByteOrderSpec)
        -> (Option<Self>, &[u8]);
}

impl Unpack for f64 {
    fn unpack(
        data: &[u8],
        data_type: DataType,
        byte_order: ByteOrderSpec,
    ) -> (Option<Self>, &[u8]) {
        let val = match byte_order {
            ByteOrderSpec::BigEndian => {
                try_unpack_data::<f64, byteorder::BigEndian>(data, data_type)
            }
            ByteOrderSpec::LittleEndian => {
                try_unpack_data::<f64, byteorder::LittleEndian>(data, data_type)
            }
        }
        .ok();
        let advanced_data = if val.is_some() {
            &data[data_type.byte_size()..]
        } else {
            data
        };
        (val, advanced_data)
    }
}

impl Unpack for f32 {
    fn unpack(
        data: &[u8],
        data_type: DataType,
        byte_order: ByteOrderSpec,
    ) -> (Option<Self>, &[u8]) {
        let val = match byte_order {
            ByteOrderSpec::BigEndian => {
                try_unpack_data::<f32, byteorder::BigEndian>(data, data_type)
            }
            ByteOrderSpec::LittleEndian => {
                try_unpack_data::<f32, byteorder::LittleEndian>(data, data_type)
            }
        }
        .ok();
        let advanced_data = if val.is_some() {
            &data[data_type.byte_size()..]
        } else {
            data
        };
        (val, advanced_data)
    }
}

#[cfg(test)]
mod tests {
    use super::super::bytes_pack_block::{BytesPackBlock, Parameters as PackParameters};
    use super::*;
    use crate::testing::StubContext;
    use approx::assert_relative_eq;

    #[test]
    fn test_bytes_unpack_1_output() {
        let mut context = StubContext::default();
        let mut pack_block = BytesPackBlock::<f64>::default();
        let mut block = BytesUnpackBlock::<2>::default();
        let spec_strings = &["I8:BigEndian"];
        let pack_parameters = PackParameters::new(spec_strings);
        let parameters = Parameters::new(spec_strings, 1000.0);

        // Test happy path
        let test_data = 42.0;
        let expected = [42.0, 1.0];
        let packed = pack_block.process(&pack_parameters, &context, test_data);
        let unpacked = block.process(&parameters, &context, packed);
        assert_eq!(unpacked, expected.as_slice());

        // Test not-stale yet but invalid data
        let unpacked = block.process(&parameters, &context, &[]);
        assert_eq!(unpacked, expected.as_slice());

        // Now it is stale
        context.time += Duration::from_secs_f64(1.1);
        let unpacked = block.process(&parameters, &context, &[]);
        assert_eq!(unpacked, [42.0, 0.0].as_slice());
    }

    #[test]
    fn test_bytes_unpack_2_outputs() {
        let mut context = StubContext::default();
        let mut pack_block = BytesPackBlock::<(f64, f64)>::default();
        let mut block = BytesUnpackBlock::<3>::default();
        let spec_strings = &["I8:BigEndian", "U64:LittleEndian"];
        let pack_parameters = PackParameters::new(spec_strings);
        let parameters = Parameters::new(spec_strings, 1000.0);

        // Test happy path
        let test_data = (-23.0, 43.0);
        let expected = [-23.0, 43.0, 1.0];
        let packed = pack_block.process(&pack_parameters, &context, test_data);
        let expected_packed: [u8; 2] = [
            0b11101001, // -23 as i8
            0b00101011,
        ];
        assert_eq!(&packed[..2], expected_packed.as_slice());
        let unpacked = block.process(&parameters, &context, packed);
        assert_eq!(unpacked, expected.as_slice());

        // Test not-stale yet but invalid data
        let unpacked = block.process(&parameters, &context, &[]);
        assert_eq!(unpacked, expected.as_slice());

        // Now it is stale
        context.time += Duration::from_secs_f64(1.1);
        let unpacked = block.process(&parameters, &context, &[]);
        assert_eq!(unpacked, [-23.0, 43.0, 0.0].as_slice());
    }

    #[test]
    fn test_bytes_unpack_7_outputs() {
        let mut context = StubContext::default();
        let mut pack_block = BytesPackBlock::<(f64, f64, f64, f64, f64, f64, f64)>::default();
        let mut block = BytesUnpackBlock::<8>::default();
        let spec_strings = &[
            "I8:BigEndian",
            "U64:LittleEndian",
            "F32:BigEndian",
            "F64:LittleEndian",
            "I32:BigEndian",
            "U16:LittleEndian",
            "F32:LittleEndian",
        ];
        let pack_parameters = PackParameters::new(spec_strings);
        let parameters = Parameters::new(spec_strings, 1000.0);

        // Test happy path
        let test_data = (-23.0, 43.0, 1.234, 3.1, 42.5, 9999.0, -7.89);
        let packed = pack_block.process(&pack_parameters, &context, test_data);
        let unpacked = block.process(&parameters, &context, packed);
        assert_relative_eq!(unpacked[0], -23.0_f64);
        assert_relative_eq!(unpacked[1], 43.0_f64);
        assert_relative_eq!(unpacked[2], 1.234_f64, epsilon = 0.001);
        assert_relative_eq!(unpacked[3], 3.1_f64);
        assert_relative_eq!(unpacked[4], 42.0_f64);
        assert_relative_eq!(unpacked[5], 9999.0_f64);
        assert_relative_eq!(unpacked[6], -7.89_f64, epsilon = 0.001);
        assert_relative_eq!(unpacked[7], 1.0_f64, epsilon = 0.001);

        // Test not-stale yet but invalid data
        let unpacked = block.process(&parameters, &context, &[]);
        assert_relative_eq!(unpacked[0], -23.0_f64);
        assert_relative_eq!(unpacked[1], 43.0_f64);
        assert_relative_eq!(unpacked[2], 1.234_f64, epsilon = 0.001);
        assert_relative_eq!(unpacked[3], 3.1_f64);
        assert_relative_eq!(unpacked[4], 42.0_f64);
        assert_relative_eq!(unpacked[5], 9999.0_f64);
        assert_relative_eq!(unpacked[6], -7.89_f64, epsilon = 0.001);
        assert_relative_eq!(unpacked[7], 1.0_f64, epsilon = 0.001);

        // Now it is stale
        context.time += Duration::from_secs_f64(1.1);
        let unpacked = block.process(&parameters, &context, &[]);
        assert_relative_eq!(unpacked[0], -23.0_f64);
        assert_relative_eq!(unpacked[1], 43.0_f64);
        assert_relative_eq!(unpacked[2], 1.234_f64, epsilon = 0.001);
        assert_relative_eq!(unpacked[3], 3.1_f64);
        assert_relative_eq!(unpacked[4], 42.0_f64);
        assert_relative_eq!(unpacked[5], 9999.0_f64);
        assert_relative_eq!(unpacked[6], -7.89_f64, epsilon = 0.001);
        assert_relative_eq!(unpacked[7], 0.0_f64, epsilon = 0.001);
    }

    #[test]
    fn test_bytes_unpack_12_outputs() {
        let context = StubContext::default();
        let mut pack_block_1 = BytesPackBlock::<(f64, f64, f64, f64, f64, f64)>::default();
        let mut pack_block_2 = BytesPackBlock::<(f64, f64, f64, f64, f64, f64)>::default();
        let mut block = BytesUnpackBlock::<13>::default();
        let spec_strings = &[
            "I8:BigEndian",
            "U64:LittleEndian",
            "F32:BigEndian",
            "F64:LittleEndian",
            "I32:BigEndian",
            "U16:LittleEndian",
            "F32:LittleEndian",
            "I8:BigEndian",
            "U64:LittleEndian",
            "F64:BigEndian",
            "F64:LittleEndian",
            "I32:BigEndian",
        ];
        let pack_parameters_1 = PackParameters::new(&spec_strings[0..6]);
        let pack_parameters_2 = PackParameters::new(&spec_strings[6..12]);
        let parameters = Parameters::new(spec_strings, 1000.0);
        // Test happy path
        let test_data_1 = (-23.0, 43.0, 1.234, 3.1, 42.5, 9999.0);
        let test_data_2 = (
            -7.89,
            127.0,
            123456789.0,
            3.4028235e38,
            2.2250739e-308,
            -2147483648.0,
        );
        let packed_1 = pack_block_1.process(&pack_parameters_1, &context, test_data_1);
        let packed_2 = pack_block_2.process(&pack_parameters_2, &context, test_data_2);
        let mut packed = alloc::vec![];
        packed.extend_from_slice(packed_1);
        packed.extend_from_slice(packed_2);
        let unpacked = block.process(&parameters, &context, packed.as_slice());
        assert_relative_eq!(unpacked[0], -23.0_f64);
        assert_relative_eq!(unpacked[1], 43.0_f64);
        assert_relative_eq!(unpacked[2], 1.234_f64, epsilon = 0.001);
        assert_relative_eq!(unpacked[3], 3.1_f64);
        assert_relative_eq!(unpacked[4], 42.0_f64);
        assert_relative_eq!(unpacked[5], 9999.0_f64);
        assert_relative_eq!(unpacked[6], -7.89_f64, epsilon = 0.001);
        assert_relative_eq!(unpacked[7], 127.0_f64);
        assert_relative_eq!(unpacked[8], 123456789.0_f64);
        assert_relative_eq!(unpacked[9], 3.4028235e38_f64, epsilon = 0.001);
        assert_relative_eq!(unpacked[10], 2.2250739e-308_f64);
        assert_relative_eq!(unpacked[11], -2147483648.0_f64);
        assert_relative_eq!(unpacked[12], 1.0_f64, epsilon = 0.001);
    }

    #[test]
    fn test_bytes_unpack_3_output_f32() {
        let mut context = StubContext::default();
        let mut pack_block = BytesPackBlock::<(f32, f32)>::default();
        let mut block = BytesUnpackBlock::<3, f32>::default();
        let spec_strings = &["I8:BigEndian", "U64:LittleEndian"];
        let pack_parameters = PackParameters::new(spec_strings);
        let parameters = Parameters::new(spec_strings, 1000.0);

        // Test happy path
        let test_data = (-23.0, 43.0);
        let expected = [-23.0, 43.0, 1.0];
        let packed = pack_block.process(&pack_parameters, &context, test_data);
        let unpacked = block.process(&parameters, &context, packed);
        assert_eq!(unpacked, expected.as_slice());

        // Test not-stale yet but invalid data
        let unpacked: &[f32; 3] = block.process(&parameters, &context, &[]);
        assert_eq!(unpacked, expected.as_slice());

        // Now it is stale
        context.time += Duration::from_secs_f64(1.1);
        let unpacked = block.process(&parameters, &context, &[]);
        assert_eq!(unpacked, [-23.0, 43.0, 0.0].as_slice());
    }
}
