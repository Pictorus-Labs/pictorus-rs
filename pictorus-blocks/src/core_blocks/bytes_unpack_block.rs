extern crate alloc;
use core::time::Duration;

use crate::{traits::Scalar, IsValid};
use alloc::{vec, vec::Vec};

use crate::byte_data::{parse_byte_data_spec, try_unpack_data, ByteOrderSpec, DataType};
use block_data::BlockData as OldBlockData;
use pictorus_traits::{ByteSliceSignal, Pass, PassBy, ProcessBlock};

pub struct BytesUnpackBlock<T: Apply> {
    pub data: Vec<OldBlockData>,
    buffer: T,
    last_valid_time: Option<Duration>,
}

impl<T: Apply> Default for BytesUnpackBlock<T> {
    fn default() -> Self {
        let buffer = T::default();
        let data = buffer.as_old_block_data();
        BytesUnpackBlock {
            data,
            buffer,
            last_valid_time: None,
        }
    }
}

impl<T: Apply> ProcessBlock for BytesUnpackBlock<T> {
    type Inputs = ByteSliceSignal;
    type Output = T;
    type Parameters = T::Parameters;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        let update_age = context.time() - self.last_valid_time.unwrap_or_default();
        let unpack_success = T::apply(&mut self.buffer, inputs, parameters, update_age);
        self.data = self.buffer.as_old_block_data();
        if unpack_success {
            self.last_valid_time = Some(context.time());
        }
        self.buffer.as_by()
    }
}

impl<T: Apply> IsValid for BytesUnpackBlock<T> {
    fn is_valid(&self, _app_time_s: f64) -> OldBlockData {
        OldBlockData::scalar_from_bool(T::is_valid(&self.buffer))
    }
}

pub struct Parameters<const N: usize> {
    pub pack_spec: [(DataType, ByteOrderSpec); N],
    pub stale_age: Duration,
}

impl<const N: usize> Parameters<N> {
    /// This constructor takes a slice of strings that represent the data spec for each input.
    pub fn new<S: AsRef<str>>(pack_spec_str: &[S], stale_age_ms: f64) -> Self {
        let pack_spec = parse_byte_data_spec(pack_spec_str)
            .try_into()
            .expect("Bytes Data Spec is incorrectly sized for the number of inputs");
        Self {
            pack_spec,
            stale_age: Duration::from_secs_f64(stale_age_ms / 1000.0),
        }
    }
}

pub trait Unpack: Scalar {
    fn unpack(data: &[u8], data_type: DataType, byte_order: ByteOrderSpec)
        -> (Option<Self>, &[u8]);

    /// Just needed to support the OldBlockData
    fn as_f64(&self) -> f64;
}

impl Unpack for f64 {
    fn unpack(
        data: &[u8],
        data_type: DataType,
        byte_order: ByteOrderSpec,
    ) -> (Option<Self>, &[u8]) {
        let val = match byte_order {
            ByteOrderSpec::BigEndian => try_unpack_data::<byteorder::BigEndian>(data, data_type),
            ByteOrderSpec::LittleEndian => {
                try_unpack_data::<byteorder::LittleEndian>(data, data_type)
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

    fn as_f64(&self) -> f64 {
        *self
    }
}

pub trait Apply: Default + Pass {
    type Parameters;

    fn apply(
        dest: &mut Self,
        data: PassBy<ByteSliceSignal>,
        parameters: &Self::Parameters,
        update_age: Duration,
    ) -> bool;

    fn as_old_block_data(&self) -> Vec<OldBlockData>;

    fn is_valid(&self) -> bool;
}

impl<T: Unpack> Apply for (T, f64) {
    type Parameters = Parameters<1>;

    fn apply(
        dest: &mut Self,
        data: PassBy<ByteSliceSignal>,
        parameters: &Self::Parameters,
        update_age: Duration,
    ) -> bool {
        let val1 = T::unpack(data, parameters.pack_spec[0].0, parameters.pack_spec[0].1).0;
        if let Some(val1) = val1 {
            *dest = (val1, 1.0);
            true
        } else {
            if update_age > parameters.stale_age {
                dest.1 = 0.0;
            }
            false
        }
    }

    fn as_old_block_data(&self) -> Vec<OldBlockData> {
        vec![OldBlockData::from_scalar(self.0.as_f64())]
    }

    fn is_valid(&self) -> bool {
        self.1.is_truthy()
    }
}

impl<T1: Unpack, T2: Unpack> Apply for (T1, T2, f64) {
    type Parameters = Parameters<2>;

    fn apply(
        dest: &mut Self,
        data: PassBy<ByteSliceSignal>,
        parameters: &Self::Parameters,
        update_age: Duration,
    ) -> bool {
        let (val1, data) = T1::unpack(data, parameters.pack_spec[0].0, parameters.pack_spec[0].1);
        let (val2, _) = T2::unpack(data, parameters.pack_spec[1].0, parameters.pack_spec[1].1);
        if let (Some(val1), Some(val2)) = (val1, val2) {
            *dest = (val1, val2, 1.0);
            true
        } else {
            if update_age > parameters.stale_age {
                dest.2 = 0.0;
            }
            false
        }
    }

    fn as_old_block_data(&self) -> Vec<OldBlockData> {
        vec![
            OldBlockData::from_scalar(self.0.as_f64()),
            OldBlockData::from_scalar(self.1.as_f64()),
        ]
    }

    fn is_valid(&self) -> bool {
        self.2.is_truthy()
    }
}

impl<T1: Unpack, T2: Unpack, T3: Unpack> Apply for (T1, T2, T3, f64) {
    type Parameters = Parameters<3>;

    fn apply(
        dest: &mut Self,
        data: PassBy<ByteSliceSignal>,
        parameters: &Self::Parameters,
        update_age: Duration,
    ) -> bool {
        let (val1, data) = T1::unpack(data, parameters.pack_spec[0].0, parameters.pack_spec[0].1);
        let (val2, data) = T2::unpack(data, parameters.pack_spec[1].0, parameters.pack_spec[1].1);
        let (val3, _) = T3::unpack(data, parameters.pack_spec[2].0, parameters.pack_spec[2].1);
        if let (Some(val1), Some(val2), Some(val3)) = (val1, val2, val3) {
            *dest = (val1, val2, val3, 1.0);
            true
        } else {
            if update_age > parameters.stale_age {
                dest.3 = 0.0;
            }
            false
        }
    }

    fn as_old_block_data(&self) -> Vec<OldBlockData> {
        vec![
            OldBlockData::from_scalar(self.0.as_f64()),
            OldBlockData::from_scalar(self.1.as_f64()),
            OldBlockData::from_scalar(self.2.as_f64()),
        ]
    }
    fn is_valid(&self) -> bool {
        self.3.is_truthy()
    }
}

impl<T1: Unpack, T2: Unpack, T3: Unpack, T4: Unpack> Apply for (T1, T2, T3, T4, f64) {
    type Parameters = Parameters<4>;

    fn apply(
        dest: &mut Self,
        data: PassBy<ByteSliceSignal>,
        parameters: &Self::Parameters,
        update_age: Duration,
    ) -> bool {
        let (val1, data) = T1::unpack(data, parameters.pack_spec[0].0, parameters.pack_spec[0].1);
        let (val2, data) = T2::unpack(data, parameters.pack_spec[1].0, parameters.pack_spec[1].1);
        let (val3, data) = T3::unpack(data, parameters.pack_spec[2].0, parameters.pack_spec[2].1);
        let (val4, _) = T4::unpack(data, parameters.pack_spec[3].0, parameters.pack_spec[3].1);
        if let (Some(val1), Some(val2), Some(val3), Some(val4)) = (val1, val2, val3, val4) {
            *dest = (val1, val2, val3, val4, 1.0);
            true
        } else {
            if update_age > parameters.stale_age {
                dest.4 = 0.0;
            }
            false
        }
    }

    fn as_old_block_data(&self) -> Vec<OldBlockData> {
        vec![
            OldBlockData::from_scalar(self.0.as_f64()),
            OldBlockData::from_scalar(self.1.as_f64()),
            OldBlockData::from_scalar(self.2.as_f64()),
            OldBlockData::from_scalar(self.3.as_f64()),
        ]
    }

    fn is_valid(&self) -> bool {
        self.4.is_truthy()
    }
}

impl<T1: Unpack, T2: Unpack, T3: Unpack, T4: Unpack, T5: Unpack> Apply
    for (T1, T2, T3, T4, T5, f64)
{
    type Parameters = Parameters<5>;

    fn apply(
        dest: &mut Self,
        data: PassBy<ByteSliceSignal>,
        parameters: &Self::Parameters,
        update_age: Duration,
    ) -> bool {
        let (val1, data) = T1::unpack(data, parameters.pack_spec[0].0, parameters.pack_spec[0].1);
        let (val2, data) = T2::unpack(data, parameters.pack_spec[1].0, parameters.pack_spec[1].1);
        let (val3, data) = T3::unpack(data, parameters.pack_spec[2].0, parameters.pack_spec[2].1);
        let (val4, data) = T4::unpack(data, parameters.pack_spec[3].0, parameters.pack_spec[3].1);
        let (val5, _) = T5::unpack(data, parameters.pack_spec[4].0, parameters.pack_spec[4].1);
        if let (Some(val1), Some(val2), Some(val3), Some(val4), Some(val5)) =
            (val1, val2, val3, val4, val5)
        {
            *dest = (val1, val2, val3, val4, val5, 1.0);
            true
        } else {
            if update_age > parameters.stale_age {
                dest.5 = 0.0;
            }
            false
        }
    }

    fn as_old_block_data(&self) -> Vec<OldBlockData> {
        vec![
            OldBlockData::from_scalar(self.0.as_f64()),
            OldBlockData::from_scalar(self.1.as_f64()),
            OldBlockData::from_scalar(self.2.as_f64()),
            OldBlockData::from_scalar(self.3.as_f64()),
            OldBlockData::from_scalar(self.4.as_f64()),
        ]
    }

    fn is_valid(&self) -> bool {
        self.5.is_truthy()
    }
}

impl<T1: Unpack, T2: Unpack, T3: Unpack, T4: Unpack, T5: Unpack, T6: Unpack> Apply
    for (T1, T2, T3, T4, T5, T6, f64)
{
    type Parameters = Parameters<6>;

    fn apply(
        dest: &mut Self,
        data: PassBy<ByteSliceSignal>,
        parameters: &Self::Parameters,
        update_age: Duration,
    ) -> bool {
        let (val1, data) = T1::unpack(data, parameters.pack_spec[0].0, parameters.pack_spec[0].1);
        let (val2, data) = T2::unpack(data, parameters.pack_spec[1].0, parameters.pack_spec[1].1);
        let (val3, data) = T3::unpack(data, parameters.pack_spec[2].0, parameters.pack_spec[2].1);
        let (val4, data) = T4::unpack(data, parameters.pack_spec[3].0, parameters.pack_spec[3].1);
        let (val5, data) = T5::unpack(data, parameters.pack_spec[4].0, parameters.pack_spec[4].1);
        let (val6, _) = T6::unpack(data, parameters.pack_spec[5].0, parameters.pack_spec[5].1);
        if let (Some(val1), Some(val2), Some(val3), Some(val4), Some(val5), Some(val6)) =
            (val1, val2, val3, val4, val5, val6)
        {
            *dest = (val1, val2, val3, val4, val5, val6, 1.0);
            true
        } else {
            if update_age > parameters.stale_age {
                dest.6 = 0.0;
            }
            false
        }
    }

    fn as_old_block_data(&self) -> Vec<OldBlockData> {
        vec![
            OldBlockData::from_scalar(self.0.as_f64()),
            OldBlockData::from_scalar(self.1.as_f64()),
            OldBlockData::from_scalar(self.2.as_f64()),
            OldBlockData::from_scalar(self.3.as_f64()),
            OldBlockData::from_scalar(self.4.as_f64()),
            OldBlockData::from_scalar(self.5.as_f64()),
        ]
    }

    fn is_valid(&self) -> bool {
        self.6.is_truthy()
    }
}

impl<T1: Unpack, T2: Unpack, T3: Unpack, T4: Unpack, T5: Unpack, T6: Unpack, T7: Unpack> Apply
    for (T1, T2, T3, T4, T5, T6, T7, f64)
{
    type Parameters = Parameters<7>;

    fn apply(
        dest: &mut Self,
        data: PassBy<ByteSliceSignal>,
        parameters: &Self::Parameters,
        update_age: Duration,
    ) -> bool {
        let (val1, data) = T1::unpack(data, parameters.pack_spec[0].0, parameters.pack_spec[0].1);
        let (val2, data) = T2::unpack(data, parameters.pack_spec[1].0, parameters.pack_spec[1].1);
        let (val3, data) = T3::unpack(data, parameters.pack_spec[2].0, parameters.pack_spec[2].1);
        let (val4, data) = T4::unpack(data, parameters.pack_spec[3].0, parameters.pack_spec[3].1);
        let (val5, data) = T5::unpack(data, parameters.pack_spec[4].0, parameters.pack_spec[4].1);
        let (val6, data) = T6::unpack(data, parameters.pack_spec[5].0, parameters.pack_spec[5].1);
        let (val7, _) = T7::unpack(data, parameters.pack_spec[6].0, parameters.pack_spec[6].1);
        if let (
            Some(val1),
            Some(val2),
            Some(val3),
            Some(val4),
            Some(val5),
            Some(val6),
            Some(val7),
        ) = (val1, val2, val3, val4, val5, val6, val7)
        {
            *dest = (val1, val2, val3, val4, val5, val6, val7, 1.0);
            true
        } else {
            if update_age > parameters.stale_age {
                dest.7 = 0.0;
            }
            false
        }
    }

    fn as_old_block_data(&self) -> Vec<OldBlockData> {
        vec![
            OldBlockData::from_scalar(self.0.as_f64()),
            OldBlockData::from_scalar(self.1.as_f64()),
            OldBlockData::from_scalar(self.2.as_f64()),
            OldBlockData::from_scalar(self.3.as_f64()),
            OldBlockData::from_scalar(self.4.as_f64()),
            OldBlockData::from_scalar(self.5.as_f64()),
            OldBlockData::from_scalar(self.6.as_f64()),
        ]
    }
    fn is_valid(&self) -> bool {
        self.7.is_truthy()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_blocks::bytes_pack_block::{BytesPackBlock, Parameters as PackParameters};
    use crate::testing::StubContext;
    use approx::assert_relative_eq;

    #[test]
    fn test_bytes_unpack_1_output() {
        let mut context = StubContext::default();
        let mut pack_block = BytesPackBlock::<f64>::default();
        let mut block = BytesUnpackBlock::<(f64, f64)>::default();
        let spec_strings = &["I8:BigEndian"];
        let pack_parameters = PackParameters::new(spec_strings);
        let parameters = Parameters::new(spec_strings, 1000.0);

        // Test happy path
        let test_data = 42.0;
        let expected = (42.0, 1.0);
        let packed = pack_block.process(&pack_parameters, &context, test_data);
        let unpacked = block.process(&parameters, &context, packed);
        assert_eq!(unpacked, expected);

        // Test not-stale yet but invalid data
        let unpacked = block.process(&parameters, &context, &[]);
        assert_eq!(unpacked, expected);

        // Now it is stale
        context.time += Duration::from_secs_f64(1.1);
        let unpacked = block.process(&parameters, &context, &[]);
        assert_eq!(unpacked, (42.0, 0.0));
    }

    #[test]
    fn test_bytes_unpack_2_outputs() {
        let mut context = StubContext::default();
        let mut pack_block = BytesPackBlock::<(f64, f64)>::default();
        let mut block = BytesUnpackBlock::<(f64, f64, f64)>::default();
        let spec_strings = &["I8:BigEndian", "U64:LittleEndian"];
        let pack_parameters = PackParameters::new(spec_strings);
        let parameters = Parameters::new(spec_strings, 1000.0);

        // Test happy path
        let test_data = (-23.0, 43.0);
        let expected = (-23.0, 43.0, 1.0);
        let packed = pack_block.process(&pack_parameters, &context, test_data);
        let unpacked = block.process(&parameters, &context, packed);
        assert_eq!(unpacked, expected);

        // Test not-stale yet but invalid data
        let unpacked = block.process(&parameters, &context, &[]);
        assert_eq!(unpacked, expected);

        // Now it is stale
        context.time += Duration::from_secs_f64(1.1);
        let unpacked = block.process(&parameters, &context, &[]);
        assert_eq!(unpacked, (-23.0, 43.0, 0.0));
    }

    #[test]
    fn test_bytes_unpack_3_outputs() {
        let mut context = StubContext::default();
        let mut pack_block = BytesPackBlock::<(f64, f64, f64)>::default();
        let mut block = BytesUnpackBlock::<(f64, f64, f64, f64)>::default();
        let spec_strings = &["I8:BigEndian", "U64:LittleEndian", "F32:BigEndian"];
        let pack_parameters = PackParameters::new(spec_strings);
        let parameters = Parameters::new(spec_strings, 1000.0);

        // Test happy path
        let test_data = (-23.0, 43.0, 1.234);
        let packed = pack_block.process(&pack_parameters, &context, test_data);
        let unpacked = block.process(&parameters, &context, packed);
        assert_relative_eq!(unpacked.0, -23.0_f64);
        assert_relative_eq!(unpacked.1, 43.0_f64);
        assert_relative_eq!(unpacked.2, 1.234_f64, epsilon = 0.001);
        assert_eq!(unpacked.3, 1.0);

        // Test not-stale yet but invalid data
        let unpacked = block.process(&parameters, &context, &[]);
        assert_relative_eq!(unpacked.0, -23.0_f64);
        assert_relative_eq!(unpacked.1, 43.0_f64);
        assert_relative_eq!(unpacked.2, 1.234_f64, epsilon = 0.001);
        assert_eq!(unpacked.3, 1.0);

        // Now it is stale
        context.time += Duration::from_secs_f64(1.1);
        let unpacked = block.process(&parameters, &context, &[]);
        assert_relative_eq!(unpacked.0, -23.0_f64);
        assert_relative_eq!(unpacked.1, 43.0_f64);
        assert_relative_eq!(unpacked.2, 1.234_f64, epsilon = 0.001);
        assert_eq!(unpacked.3, 0.0);
    }

    #[test]
    fn test_bytes_unpack_4_outputs() {
        let mut context = StubContext::default();
        let mut pack_block = BytesPackBlock::<(f64, f64, f64, f64)>::default();
        let mut block = BytesUnpackBlock::<(f64, f64, f64, f64, f64)>::default();
        let spec_strings = &[
            "I8:BigEndian",
            "U64:LittleEndian",
            "F32:BigEndian",
            "F64:LittleEndian",
        ];
        let pack_parameters = PackParameters::new(spec_strings);
        let parameters = Parameters::new(spec_strings, 1000.0);

        // Test happy path
        let test_data = (-23.0, 43.0, 1.234, 3.1);
        let packed = pack_block.process(&pack_parameters, &context, test_data);
        let unpacked = block.process(&parameters, &context, packed);
        assert_relative_eq!(unpacked.0, -23.0_f64);
        assert_relative_eq!(unpacked.1, 43.0_f64);
        assert_relative_eq!(unpacked.2, 1.234_f64, epsilon = 0.001);
        assert_relative_eq!(unpacked.3, 3.1_f64);
        assert_eq!(unpacked.4, 1.0);

        // Test not-stale yet but invalid data
        let unpacked = block.process(&parameters, &context, &[]);
        assert_relative_eq!(unpacked.0, -23.0_f64);
        assert_relative_eq!(unpacked.1, 43.0_f64);
        assert_relative_eq!(unpacked.2, 1.234_f64, epsilon = 0.001);
        assert_relative_eq!(unpacked.3, 3.1_f64);
        assert_eq!(unpacked.4, 1.0);

        // Now it is stale
        context.time += Duration::from_secs_f64(1.1);
        let unpacked = block.process(&parameters, &context, &[]);
        assert_relative_eq!(unpacked.0, -23.0_f64);
        assert_relative_eq!(unpacked.1, 43.0_f64);
        assert_relative_eq!(unpacked.2, 1.234_f64, epsilon = 0.001);
        assert_relative_eq!(unpacked.3, 3.1);
        assert_eq!(unpacked.4, 0.0);
    }

    #[test]
    fn test_bytes_unpack_5_outputs() {
        let mut context = StubContext::default();
        let mut pack_block = BytesPackBlock::<(f64, f64, f64, f64, f64)>::default();
        let mut block = BytesUnpackBlock::<(f64, f64, f64, f64, f64, f64)>::default();
        let spec_strings = &[
            "I8:BigEndian",
            "U64:LittleEndian",
            "F32:BigEndian",
            "F64:LittleEndian",
            "I32:BigEndian",
        ];
        let pack_parameters = PackParameters::new(spec_strings);
        let parameters = Parameters::new(spec_strings, 1000.0);

        // Test happy path
        let test_data = (-23.0, 43.0, 1.234, 3.1, 42.5);
        let packed = pack_block.process(&pack_parameters, &context, test_data);
        let unpacked = block.process(&parameters, &context, packed);
        assert_relative_eq!(unpacked.0, -23.0_f64);
        assert_relative_eq!(unpacked.1, 43.0_f64);
        assert_relative_eq!(unpacked.2, 1.234_f64, epsilon = 0.001);
        assert_relative_eq!(unpacked.3, 3.1_f64);
        assert_relative_eq!(unpacked.4, 42.0_f64);
        assert_eq!(unpacked.5, 1.0);

        // Test not-stale yet but invalid data
        let unpacked = block.process(&parameters, &context, &[]);
        assert_relative_eq!(unpacked.0, -23.0_f64);
        assert_relative_eq!(unpacked.1, 43.0_f64);
        assert_relative_eq!(unpacked.2, 1.234_f64, epsilon = 0.001);
        assert_relative_eq!(unpacked.3, 3.1_f64);
        assert_relative_eq!(unpacked.4, 42.0_f64);
        assert_eq!(unpacked.5, 1.0);

        // Now it is stale
        context.time += Duration::from_secs_f64(1.1);
        let unpacked = block.process(&parameters, &context, &[]);
        assert_relative_eq!(unpacked.0, -23.0_f64);
        assert_relative_eq!(unpacked.1, 43.0_f64);
        assert_relative_eq!(unpacked.2, 1.234_f64, epsilon = 0.001);
        assert_relative_eq!(unpacked.3, 3.1_f64);
        assert_relative_eq!(unpacked.4, 42.0_f64);
        assert_eq!(unpacked.5, 0.0);
    }

    #[test]
    fn test_bytes_unpack_6_outputs() {
        let mut context = StubContext::default();
        let mut pack_block = BytesPackBlock::<(f64, f64, f64, f64, f64, f64)>::default();
        let mut block = BytesUnpackBlock::<(f64, f64, f64, f64, f64, f64, f64)>::default();
        let spec_strings = &[
            "I8:BigEndian",
            "U64:LittleEndian",
            "F32:BigEndian",
            "F64:LittleEndian",
            "I32:BigEndian",
            "U16:LittleEndian",
        ];
        let pack_parameters = PackParameters::new(spec_strings);
        let parameters = Parameters::new(spec_strings, 1000.0);

        // Test happy path
        let test_data = (-23.0, 43.0, 1.234, 3.1, 42.5, 9999.0);
        let packed = pack_block.process(&pack_parameters, &context, test_data);
        let unpacked = block.process(&parameters, &context, packed);
        assert_relative_eq!(unpacked.0, -23.0_f64);
        assert_relative_eq!(unpacked.1, 43.0_f64);
        assert_relative_eq!(unpacked.2, 1.234_f64, epsilon = 0.001);
        assert_relative_eq!(unpacked.3, 3.1_f64);
        assert_relative_eq!(unpacked.4, 42.0_f64);
        assert_relative_eq!(unpacked.5, 9999.0_f64);
        assert_eq!(unpacked.6, 1.0);

        // Test not-stale yet but invalid data
        let unpacked = block.process(&parameters, &context, &[]);
        assert_relative_eq!(unpacked.0, -23.0_f64);
        assert_relative_eq!(unpacked.1, 43.0_f64);
        assert_relative_eq!(unpacked.2, 1.234_f64, epsilon = 0.001);
        assert_relative_eq!(unpacked.3, 3.1_f64);
        assert_relative_eq!(unpacked.4, 42.0_f64);
        assert_relative_eq!(unpacked.5, 9999.0_f64);
        assert_eq!(unpacked.6, 1.0);

        // Now it is stale
        context.time += Duration::from_secs_f64(1.1);
        let unpacked = block.process(&parameters, &context, &packed[..15]);
        assert_relative_eq!(unpacked.0, -23.0_f64);
        assert_relative_eq!(unpacked.1, 43.0_f64);
        assert_relative_eq!(unpacked.2, 1.234_f64, epsilon = 0.001);
        assert_relative_eq!(unpacked.3, 3.1_f64);
        assert_relative_eq!(unpacked.4, 42.0_f64);
        assert_relative_eq!(unpacked.5, 9999.0_f64);
        assert_eq!(unpacked.6, 0.0);

        // Make it un-stale with new input
        let test_data = (1337.0, 12.0, 1994.0, -8.3, 71.92, -15.0);
        let packed = pack_block.process(&pack_parameters, &context, test_data);
        let unpacked = block.process(&parameters, &context, packed);
        assert_relative_eq!(unpacked.0, 127.0_f64); //I8 Max
        assert_relative_eq!(unpacked.1, 12.0_f64);
        assert_relative_eq!(unpacked.2, 1994.0_f64);
        assert_relative_eq!(unpacked.3, -8.3_f64);
        assert_relative_eq!(unpacked.4, 71.0_f64); // Int storage drops decimal
        assert_relative_eq!(unpacked.5, 0.0_f64); // unsigned storage can't hold negative and defaults to 0
        assert_eq!(unpacked.6, 1.0);
    }

    #[test]
    fn test_bytes_unpack_7_outputs() {
        let mut context = StubContext::default();
        let mut pack_block = BytesPackBlock::<(f64, f64, f64, f64, f64, f64, f64)>::default();
        let mut block = BytesUnpackBlock::<(f64, f64, f64, f64, f64, f64, f64, f64)>::default();
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
        assert_relative_eq!(unpacked.0, -23.0_f64);
        assert_relative_eq!(unpacked.1, 43.0_f64);
        assert_relative_eq!(unpacked.2, 1.234_f64, epsilon = 0.001);
        assert_relative_eq!(unpacked.3, 3.1_f64);
        assert_relative_eq!(unpacked.4, 42.0_f64);
        assert_relative_eq!(unpacked.5, 9999.0_f64);
        assert_relative_eq!(unpacked.6, -7.89_f64, epsilon = 0.001);
        assert_eq!(unpacked.7, 1.0);

        // Test not-stale yet but invalid data
        let unpacked = block.process(&parameters, &context, &[]);
        assert_relative_eq!(unpacked.0, -23.0_f64);
        assert_relative_eq!(unpacked.1, 43.0_f64);
        assert_relative_eq!(unpacked.2, 1.234_f64, epsilon = 0.001);
        assert_relative_eq!(unpacked.3, 3.1_f64);
        assert_relative_eq!(unpacked.4, 42.0_f64);
        assert_relative_eq!(unpacked.5, 9999.0_f64);
        assert_relative_eq!(unpacked.6, -7.89_f64, epsilon = 0.001);
        assert_eq!(unpacked.7, 1.0);

        // Now it is stale
        context.time += Duration::from_secs_f64(1.1);
        let unpacked = block.process(&parameters, &context, &[]);
        assert_relative_eq!(unpacked.0, -23.0_f64);
        assert_relative_eq!(unpacked.1, 43.0_f64);
        assert_relative_eq!(unpacked.2, 1.234_f64, epsilon = 0.001);
        assert_relative_eq!(unpacked.3, 3.1_f64);
        assert_relative_eq!(unpacked.4, 42.0_f64);
        assert_relative_eq!(unpacked.5, 9999.0_f64);
        assert_relative_eq!(unpacked.6, -7.89_f64, epsilon = 0.001);
        assert_eq!(unpacked.7, 0.0);
    }
}
