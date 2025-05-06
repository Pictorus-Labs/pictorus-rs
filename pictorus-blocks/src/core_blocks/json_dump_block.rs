extern crate alloc;
use crate::traits::serialize::{ByteSliceFormat, Serialize};
use alloc::{string::String, vec::Vec};
use block_data::BlockData as OldBlockData;
use miniserde::json::{self, Value};
use pictorus_traits::{ByteSliceSignal, Pass, PassBy, ProcessBlock};

/// This block can be used to take a set of input signals and serialize them into a JSON object.
/// That object is then serialized into a byte slice to be returned
pub struct JsonDumpBlock<T: Apply> {
    pub data: OldBlockData,
    buffer: Vec<u8>,
    _phantom: core::marker::PhantomData<T>,
}

impl<T: Apply> Default for JsonDumpBlock<T> {
    fn default() -> Self {
        Self {
            data: OldBlockData::from_bytes(b""),
            buffer: Vec::new(),
            _phantom: core::marker::PhantomData,
        }
    }
}

impl<T: Apply> ProcessBlock for JsonDumpBlock<T> {
    type Inputs = T;
    type Output = ByteSliceSignal;
    type Parameters = Parameters;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        T::apply(&mut self.buffer, inputs, parameters);
        self.data = OldBlockData::from_bytes(&self.buffer);
        &self.buffer
    }
}

/// Option for how a specific signal should be encoded
#[derive(Debug, Clone, Copy, PartialEq, strum::EnumString)]
pub enum EncodingType {
    /// Just normal JSON encoding
    /// e.g. `42.0_f64` => `42.0`
    /// e.g. `Matrix{ data: [[1.0, 3.0],[2.0, 4.0]] }` => `[[1.0, 2.0],[3.0, 4.0]]` (note that [`Matrix`] internally is col-major but we serialize it as row major)
    /// For [`ByteSliceSignal`] this will encode the bytes as an array of numbers
    /// (e.g. `[104,101,108,108,111]` => `[104, 101, 108, 108, 111]`)
    Default,
    /// Encode the signal as a UTF-8 string (e.g. `42.0_f64` => `"42.0"`)
    /// For [`ByteSliceSignal`] this will encode the bytes as a string
    /// (e.g. `[104,101,108,108,111]` => `"hello"`)
    Utf8,
}

/// Parameters for the JsonDumpBlock
/// The parameters are used to specify how each signal should be encoded and what its key name should be
/// in the JSON object.
#[derive(Debug, Clone, PartialEq)]
pub struct Parameters {
    /// The key name and encoding type for each data item
    ///
    /// TODO: The keynames should probably be a `&'static str` but that's not possible with the current
    /// codegen.
    pub encoding_spec: Vec<(EncodingType, String)>,
}

impl Parameters {
    pub fn new(encoding_spec: &[String]) -> Self {
        let encoding_spec = Self::parse_output_spec(encoding_spec);
        Self { encoding_spec }
    }

    fn parse_output_spec(data: &[String]) -> Vec<(EncodingType, String)> {
        data.iter()
            .map(|d| d.split_once(':').expect("Invalid output data format"))
            .map(|(dt, field)| (dt.parse::<EncodingType>().unwrap(), field.into()))
            .collect()
    }
}

/// A trait for transforming a type into a JSON value.
/// We must wrap the `Serialize` trait because we need to specify the encoding type
/// as defined for this block.
pub trait AsJson: Pass {
    fn as_json(input: PassBy<Self>, encoding: EncodingType) -> Value;
}

impl<S: Serialize<FormatOptions = ()>> AsJson for S {
    fn as_json(input: PassBy<Self>, encoding: EncodingType) -> Value {
        let data = S::as_json_value(input, ());
        if encoding == EncodingType::Utf8 {
            Value::String(json::to_string(&data))
        } else {
            data
        }
    }
}

impl AsJson for ByteSliceSignal {
    fn as_json(input: PassBy<Self>, encoding: EncodingType) -> Value {
        let format_option = match encoding {
            EncodingType::Default => ByteSliceFormat::Array,
            EncodingType::Utf8 => ByteSliceFormat::String,
        };
        ByteSliceSignal::as_json_value(input, format_option)
    }
}

pub trait Apply: Pass {
    fn apply(dest: &mut Vec<u8>, input: PassBy<Self>, parameters: &Parameters);
}

impl<T1> Apply for T1
where
    T1: AsJson,
{
    fn apply(dest: &mut Vec<u8>, input: PassBy<Self>, parameters: &Parameters) {
        dest.clear();
        let json_value = if parameters.encoding_spec.is_empty() {
            T1::as_json(input, EncodingType::Default)
        } else {
            let json_value = T1::as_json(input, parameters.encoding_spec[0].0);
            let mut data = json::Object::new();
            data.insert(parameters.encoding_spec[0].1.clone(), json_value);
            Value::Object(data)
        };
        dest.extend_from_slice(json::to_string(&json_value).as_bytes());
    }
}

impl<T1, T2> Apply for (T1, T2)
where
    T1: AsJson,
    T2: AsJson,
{
    fn apply(dest: &mut Vec<u8>, input: PassBy<Self>, parameters: &Parameters) {
        assert!(parameters.encoding_spec.len() == 2 || parameters.encoding_spec.is_empty());
        dest.clear();
        let json_value = if parameters.encoding_spec.is_empty() {
            let mut data = json::Array::new();
            data.push(T1::as_json(input.0, EncodingType::Default));
            data.push(T2::as_json(input.1, EncodingType::Default));
            Value::Array(data)
        } else {
            let mut data = json::Object::new();
            data.insert(
                parameters.encoding_spec[0].1.clone(),
                T1::as_json(input.0, parameters.encoding_spec[0].0),
            );
            data.insert(
                parameters.encoding_spec[1].1.clone(),
                T2::as_json(input.1, parameters.encoding_spec[1].0),
            );
            Value::Object(data)
        };
        dest.extend_from_slice(json::to_string(&json_value).as_bytes());
    }
}

impl<T1, T2, T3> Apply for (T1, T2, T3)
where
    T1: AsJson,
    T2: AsJson,
    T3: AsJson,
{
    fn apply(dest: &mut Vec<u8>, input: PassBy<Self>, parameters: &Parameters) {
        assert!(parameters.encoding_spec.len() == 3 || parameters.encoding_spec.is_empty());
        dest.clear();
        let json_value = if parameters.encoding_spec.is_empty() {
            let mut data = json::Array::new();
            data.push(T1::as_json(input.0, EncodingType::Default));
            data.push(T2::as_json(input.1, EncodingType::Default));
            data.push(T3::as_json(input.2, EncodingType::Default));
            Value::Array(data)
        } else {
            let mut data = json::Object::new();
            data.insert(
                parameters.encoding_spec[0].1.clone(),
                T1::as_json(input.0, parameters.encoding_spec[0].0),
            );
            data.insert(
                parameters.encoding_spec[1].1.clone(),
                T2::as_json(input.1, parameters.encoding_spec[1].0),
            );
            data.insert(
                parameters.encoding_spec[2].1.clone(),
                T3::as_json(input.2, parameters.encoding_spec[2].0),
            );
            Value::Object(data)
        };
        dest.extend_from_slice(json::to_string(&json_value).as_bytes());
    }
}

impl<T1, T2, T3, T4> Apply for (T1, T2, T3, T4)
where
    T1: AsJson,
    T2: AsJson,
    T3: AsJson,
    T4: AsJson,
{
    fn apply(dest: &mut Vec<u8>, input: PassBy<Self>, parameters: &Parameters) {
        assert!(parameters.encoding_spec.len() == 4 || parameters.encoding_spec.is_empty());
        dest.clear();
        let json_value = if parameters.encoding_spec.is_empty() {
            let mut data = json::Array::new();
            data.push(T1::as_json(input.0, EncodingType::Default));
            data.push(T2::as_json(input.1, EncodingType::Default));
            data.push(T3::as_json(input.2, EncodingType::Default));
            data.push(T4::as_json(input.3, EncodingType::Default));
            Value::Array(data)
        } else {
            let mut data = json::Object::new();
            data.insert(
                parameters.encoding_spec[0].1.clone(),
                T1::as_json(input.0, parameters.encoding_spec[0].0),
            );
            data.insert(
                parameters.encoding_spec[1].1.clone(),
                T2::as_json(input.1, parameters.encoding_spec[1].0),
            );
            data.insert(
                parameters.encoding_spec[2].1.clone(),
                T3::as_json(input.2, parameters.encoding_spec[2].0),
            );
            data.insert(
                parameters.encoding_spec[3].1.clone(),
                T4::as_json(input.3, parameters.encoding_spec[3].0),
            );
            Value::Object(data)
        };
        dest.extend_from_slice(json::to_string(&json_value).as_bytes());
    }
}

impl<T1, T2, T3, T4, T5> Apply for (T1, T2, T3, T4, T5)
where
    T1: AsJson,
    T2: AsJson,
    T3: AsJson,
    T4: AsJson,
    T5: AsJson,
{
    fn apply(dest: &mut Vec<u8>, input: PassBy<Self>, parameters: &Parameters) {
        assert!(parameters.encoding_spec.len() == 5 || parameters.encoding_spec.is_empty());
        dest.clear();
        let json_value = if parameters.encoding_spec.is_empty() {
            let mut data = json::Array::new();
            data.push(T1::as_json(input.0, EncodingType::Default));
            data.push(T2::as_json(input.1, EncodingType::Default));
            data.push(T3::as_json(input.2, EncodingType::Default));
            data.push(T4::as_json(input.3, EncodingType::Default));
            data.push(T5::as_json(input.4, EncodingType::Default));
            Value::Array(data)
        } else {
            let mut data = json::Object::new();
            data.insert(
                parameters.encoding_spec[0].1.clone(),
                T1::as_json(input.0, parameters.encoding_spec[0].0),
            );
            data.insert(
                parameters.encoding_spec[1].1.clone(),
                T2::as_json(input.1, parameters.encoding_spec[1].0),
            );
            data.insert(
                parameters.encoding_spec[2].1.clone(),
                T3::as_json(input.2, parameters.encoding_spec[2].0),
            );
            data.insert(
                parameters.encoding_spec[3].1.clone(),
                T4::as_json(input.3, parameters.encoding_spec[3].0),
            );
            data.insert(
                parameters.encoding_spec[4].1.clone(),
                T5::as_json(input.4, parameters.encoding_spec[4].0),
            );
            Value::Object(data)
        };
        dest.extend_from_slice(json::to_string(&json_value).as_bytes());
    }
}

impl<T1, T2, T3, T4, T5, T6> Apply for (T1, T2, T3, T4, T5, T6)
where
    T1: AsJson,
    T2: AsJson,
    T3: AsJson,
    T4: AsJson,
    T5: AsJson,
    T6: AsJson,
{
    fn apply(dest: &mut Vec<u8>, input: PassBy<Self>, parameters: &Parameters) {
        assert!(parameters.encoding_spec.len() == 6 || parameters.encoding_spec.is_empty());
        dest.clear();
        let json_value = if parameters.encoding_spec.is_empty() {
            let mut data = json::Array::new();
            data.push(T1::as_json(input.0, EncodingType::Default));
            data.push(T2::as_json(input.1, EncodingType::Default));
            data.push(T3::as_json(input.2, EncodingType::Default));
            data.push(T4::as_json(input.3, EncodingType::Default));
            data.push(T5::as_json(input.4, EncodingType::Default));
            data.push(T6::as_json(input.5, EncodingType::Default));
            Value::Array(data)
        } else {
            let mut data = json::Object::new();
            data.insert(
                parameters.encoding_spec[0].1.clone(),
                T1::as_json(input.0, parameters.encoding_spec[0].0),
            );
            data.insert(
                parameters.encoding_spec[1].1.clone(),
                T2::as_json(input.1, parameters.encoding_spec[1].0),
            );
            data.insert(
                parameters.encoding_spec[2].1.clone(),
                T3::as_json(input.2, parameters.encoding_spec[2].0),
            );
            data.insert(
                parameters.encoding_spec[3].1.clone(),
                T4::as_json(input.3, parameters.encoding_spec[3].0),
            );
            data.insert(
                parameters.encoding_spec[4].1.clone(),
                T5::as_json(input.4, parameters.encoding_spec[4].0),
            );
            data.insert(
                parameters.encoding_spec[5].1.clone(),
                T6::as_json(input.5, parameters.encoding_spec[5].0),
            );
            Value::Object(data)
        };
        dest.extend_from_slice(json::to_string(&json_value).as_bytes());
    }
}

impl<T1, T2, T3, T4, T5, T6, T7> Apply for (T1, T2, T3, T4, T5, T6, T7)
where
    T1: AsJson,
    T2: AsJson,
    T3: AsJson,
    T4: AsJson,
    T5: AsJson,
    T6: AsJson,
    T7: AsJson,
{
    fn apply(dest: &mut Vec<u8>, input: PassBy<Self>, parameters: &Parameters) {
        assert!(parameters.encoding_spec.len() == 7 || parameters.encoding_spec.is_empty());
        dest.clear();
        let json_value = if parameters.encoding_spec.is_empty() {
            let mut data = json::Array::new();
            data.push(T1::as_json(input.0, EncodingType::Default));
            data.push(T2::as_json(input.1, EncodingType::Default));
            data.push(T3::as_json(input.2, EncodingType::Default));
            data.push(T4::as_json(input.3, EncodingType::Default));
            data.push(T5::as_json(input.4, EncodingType::Default));
            data.push(T6::as_json(input.5, EncodingType::Default));
            data.push(T7::as_json(input.6, EncodingType::Default));
            Value::Array(data)
        } else {
            let mut data = json::Object::new();
            data.insert(
                parameters.encoding_spec[0].1.clone(),
                T1::as_json(input.0, parameters.encoding_spec[0].0),
            );
            data.insert(
                parameters.encoding_spec[1].1.clone(),
                T2::as_json(input.1, parameters.encoding_spec[1].0),
            );
            data.insert(
                parameters.encoding_spec[2].1.clone(),
                T3::as_json(input.2, parameters.encoding_spec[2].0),
            );
            data.insert(
                parameters.encoding_spec[3].1.clone(),
                T4::as_json(input.3, parameters.encoding_spec[3].0),
            );
            data.insert(
                parameters.encoding_spec[4].1.clone(),
                T5::as_json(input.4, parameters.encoding_spec[4].0),
            );
            data.insert(
                parameters.encoding_spec[5].1.clone(),
                T6::as_json(input.5, parameters.encoding_spec[5].0),
            );
            data.insert(
                parameters.encoding_spec[6].1.clone(),
                T7::as_json(input.6, parameters.encoding_spec[6].0),
            );
            Value::Object(data)
        };
        dest.extend_from_slice(json::to_string(&json_value).as_bytes());
    }
}

impl<T1, T2, T3, T4, T5, T6, T7, T8> Apply for (T1, T2, T3, T4, T5, T6, T7, T8)
where
    T1: AsJson,
    T2: AsJson,
    T3: AsJson,
    T4: AsJson,
    T5: AsJson,
    T6: AsJson,
    T7: AsJson,
    T8: AsJson,
{
    fn apply(dest: &mut Vec<u8>, input: PassBy<Self>, parameters: &Parameters) {
        assert!(parameters.encoding_spec.len() == 8 || parameters.encoding_spec.is_empty());
        dest.clear();
        let json_value = if parameters.encoding_spec.is_empty() {
            let mut data = json::Array::new();
            data.push(T1::as_json(input.0, EncodingType::Default));
            data.push(T2::as_json(input.1, EncodingType::Default));
            data.push(T3::as_json(input.2, EncodingType::Default));
            data.push(T4::as_json(input.3, EncodingType::Default));
            data.push(T5::as_json(input.4, EncodingType::Default));
            data.push(T6::as_json(input.5, EncodingType::Default));
            data.push(T7::as_json(input.6, EncodingType::Default));
            data.push(T8::as_json(input.7, EncodingType::Default));
            Value::Array(data)
        } else {
            let mut data = json::Object::new();
            data.insert(
                parameters.encoding_spec[0].1.clone(),
                T1::as_json(input.0, parameters.encoding_spec[0].0),
            );
            data.insert(
                parameters.encoding_spec[1].1.clone(),
                T2::as_json(input.1, parameters.encoding_spec[1].0),
            );
            data.insert(
                parameters.encoding_spec[2].1.clone(),
                T3::as_json(input.2, parameters.encoding_spec[2].0),
            );
            data.insert(
                parameters.encoding_spec[3].1.clone(),
                T4::as_json(input.3, parameters.encoding_spec[3].0),
            );
            data.insert(
                parameters.encoding_spec[4].1.clone(),
                T5::as_json(input.4, parameters.encoding_spec[4].0),
            );
            data.insert(
                parameters.encoding_spec[5].1.clone(),
                T6::as_json(input.5, parameters.encoding_spec[5].0),
            );
            data.insert(
                parameters.encoding_spec[6].1.clone(),
                T7::as_json(input.6, parameters.encoding_spec[6].0),
            );
            data.insert(
                parameters.encoding_spec[7].1.clone(),
                T8::as_json(input.7, parameters.encoding_spec[7].0),
            );
            Value::Object(data)
        };
        dest.extend_from_slice(json::to_string(&json_value).as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::ToOwned;

    use super::*;
    use crate::testing::StubContext;
    use alloc::vec;
    use miniserde::json::{self, Number};
    use pictorus_traits::Matrix;

    #[test]
    fn test_writes_object_data_if_has_labels() {
        let ctxt = StubContext::default();

        // Two floats with Default encoding
        let mut block = JsonDumpBlock::<(f64, f32)>::default();
        let parameters = Parameters::new(&["Default:foo".to_owned(), "Default:bar".to_owned()]);
        let output = block.process(&parameters, &ctxt, (1.0, 2.0));
        let expected = {
            let mut data = json::Object::new();
            data.insert("foo".to_owned(), Value::Number(Number::F64(1.0)));
            data.insert("bar".to_owned(), Value::Number(Number::F64(2.0)));
            json::to_string(&data)
        };
        assert_eq!(String::from_utf8(output.to_owned()).unwrap(), expected);

        // Two floats with mixed encoding
        let parameters = Parameters::new(&["Utf8:foo".to_owned(), "Default:bar".to_owned()]);
        let output = block.process(&parameters, &ctxt, (1.0, 2.0));
        let expected = {
            let mut data = json::Object::new();
            data.insert("foo".to_owned(), Value::String("1.0".to_owned()));
            data.insert("bar".to_owned(), Value::Number(Number::F64(2.0)));
            json::to_string(&data)
        };
        assert_eq!(String::from_utf8(output.to_owned()).unwrap(), expected);
    }

    #[test]
    fn test_array_output_without_labels() {
        let ctxt = StubContext::default();

        // Test single scalar value
        let mut block = JsonDumpBlock::<i8>::default();
        let parameters = Parameters::new(&[]);
        let output = block.process(&parameters, &ctxt, 42);
        let expected = json::to_string(&42);
        assert_eq!(output, expected.as_bytes());

        // Test two scalar values (tuple)
        let mut block = JsonDumpBlock::<(f64, f32)>::default();
        let output = block.process(&parameters, &ctxt, (1.0, 2.0));
        let expected = json::to_string(&[1.0, 2.0]);
        assert_eq!(output, expected.as_bytes());

        // Test byte slice
        let mut block = JsonDumpBlock::<ByteSliceSignal>::default();
        let output = block.process(&parameters, &ctxt, b"hello".as_ref());
        let expected = {
            let data: Vec<Value> = b"hello"
                .iter()
                .map(|b| Value::Number(Number::U64(u64::from(*b))))
                .collect();
            json::to_string(&data)
        };
        assert_eq!(String::from_utf8(output.to_vec()).unwrap(), expected);

        // Test mixed types
        let mut block = JsonDumpBlock::<(ByteSliceSignal, f64)>::default();
        let output = block.process(&parameters, &ctxt, (b"hello".as_ref(), 42.0));
        let expected = {
            let mut data = json::Array::new();
            data.push(Value::Array(
                b"hello"
                    .iter()
                    .map(|b| Value::Number(Number::U64(u64::from(*b))))
                    .collect(),
            ));
            data.push(Value::Number(Number::F64(42.0)));
            json::to_string(&data)
        };
        assert_eq!(String::from_utf8(output.to_vec()).unwrap(), expected);

        // Test with 3 elements
        let mut block = JsonDumpBlock::<(f64, f32, f32)>::default();
        let output = block.process(&parameters, &ctxt, (1.0, 2.0, 3.0));
        let expected = json::to_string(&[1.0, 2.0, 3.0]);
        assert_eq!(output, expected.as_bytes());

        // Test with 4 elements
        let mut block = JsonDumpBlock::<(f64, f32, f32, f64)>::default();
        let output = block.process(&parameters, &ctxt, (1.0, 2.0, 3.0, 4.0));
        let expected = json::to_string(&[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(output, expected.as_bytes());

        // Test with 5 elements
        let mut block = JsonDumpBlock::<(f64, f32, f32, f64, f32)>::default();
        let output = block.process(&parameters, &ctxt, (1.0, 2.0, 3.0, 4.0, 5.0));
        let expected = json::to_string(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(output, expected.as_bytes());

        // Test with 6 elements
        let mut block = JsonDumpBlock::<(f64, f32, f32, f64, f32, f64)>::default();
        let output = block.process(&parameters, &ctxt, (1.0, 2.0, 3.0, 4.0, 5.0, 6.0));
        let expected = json::to_string(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        assert_eq!(output, expected.as_bytes());

        // Test with 7 elements
        let mut block = JsonDumpBlock::<(f64, f32, f32, f64, f32, f64, f32)>::default();
        let output = block.process(&parameters, &ctxt, (1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0));
        let expected = json::to_string(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]);
        assert_eq!(output, expected.as_bytes());

        // Test with 8 elements (maximum supported)
        let mut block = JsonDumpBlock::<(i8, i16, i32, i32, u8, u16, u32, u8)>::default();
        let output = block.process(&parameters, &ctxt, (1, 2, 3, 4, 5, 6, 7, 8));
        let expected = json::to_string(&[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(output, expected.as_bytes());
    }

    #[test]
    fn test_object_output_with_labels() {
        let ctxt = StubContext::default();

        // Test single scalar value with label
        let mut block = JsonDumpBlock::<i8>::default();
        let parameters = Parameters::new(&["Default:value".to_owned()]);
        let output = block.process(&parameters, &ctxt, 42);
        let expected = {
            let mut data = json::Object::new();
            data.insert("value".to_owned(), Value::Number(Number::I64(42)));
            json::to_string(&data)
        };
        assert_eq!(String::from_utf8(output.to_vec()).unwrap(), expected);

        // Test two scalar values with labels
        let mut block = JsonDumpBlock::<(f64, f32)>::default();
        let parameters = Parameters::new(&["Default:foo".to_owned(), "Default:bar".to_owned()]);
        let output = block.process(&parameters, &ctxt, (1.0, 2.0));
        let expected = {
            let mut data = json::Object::new();
            data.insert("foo".to_owned(), Value::Number(Number::F64(1.0)));
            data.insert("bar".to_owned(), Value::Number(Number::F64(2.0)));
            json::to_string(&data)
        };
        assert_eq!(String::from_utf8(output.to_vec()).unwrap(), expected);

        // Test with mixed encoding types
        let parameters = Parameters::new(&["Utf8:foo".to_owned(), "Default:bar".to_owned()]);
        let output = block.process(&parameters, &ctxt, (1.0, 2.0));
        let expected = {
            let mut data = json::Object::new();
            data.insert("foo".to_owned(), Value::String("1.0".to_owned()));
            data.insert("bar".to_owned(), Value::Number(Number::F64(2.0)));
            json::to_string(&data)
        };
        assert_eq!(String::from_utf8(output.to_vec()).unwrap(), expected);

        // Test with all Utf8 encoding
        let parameters = Parameters::new(&["Utf8:foo".to_owned(), "Utf8:bar".to_owned()]);
        let output = block.process(&parameters, &ctxt, (1.0, 2.0));
        let expected = {
            let mut data = json::Object::new();
            data.insert("foo".to_owned(), Value::String("1.0".to_owned()));
            data.insert("bar".to_owned(), Value::String("2.0".to_owned()));
            json::to_string(&data)
        };
        assert_eq!(String::from_utf8(output.to_vec()).unwrap(), expected);

        // Test with 3 elements
        let mut block = JsonDumpBlock::<(f64, f32, f32)>::default();
        let parameters = Parameters::new(&[
            "Default:foo".to_owned(),
            "Default:bar".to_owned(),
            "Default:baz".to_owned(),
        ]);
        let output = block.process(&parameters, &ctxt, (1.0, 2.0, 3.0));
        let expected = {
            let mut data = json::Object::new();
            data.insert("foo".to_owned(), Value::Number(Number::F64(1.0)));
            data.insert("bar".to_owned(), Value::Number(Number::F64(2.0)));
            data.insert("baz".to_owned(), Value::Number(Number::F64(3.0)));
            json::to_string(&data)
        };
        assert_eq!(String::from_utf8(output.to_vec()).unwrap(), expected);

        // Test with 4 elements
        let mut block = JsonDumpBlock::<(f64, f32, f32, i32)>::default();
        let parameters = Parameters::new(&[
            "Default:one".to_owned(),
            "Default:two".to_owned(),
            "Default:three".to_owned(),
            "Default:four".to_owned(),
        ]);
        let output = block.process(&parameters, &ctxt, (1.0, 2.0, 3.0, 4));
        let expected = {
            let mut data = json::Object::new();
            data.insert("one".to_owned(), Value::Number(Number::F64(1.0)));
            data.insert("two".to_owned(), Value::Number(Number::F64(2.0)));
            data.insert("three".to_owned(), Value::Number(Number::F64(3.0)));
            data.insert("four".to_owned(), Value::Number(Number::I64(4)));
            json::to_string(&data)
        };
        assert_eq!(String::from_utf8(output.to_vec()).unwrap(), expected);

        // Test with 5 elements
        let mut block = JsonDumpBlock::<(f64, f32, f32, i32, bool)>::default();
        let parameters = Parameters::new(&[
            "Default:one".to_owned(),
            "Default:two".to_owned(),
            "Default:three".to_owned(),
            "Default:four".to_owned(),
            "Default:five".to_owned(),
        ]);
        let output = block.process(&parameters, &ctxt, (1.0, 2.0, 3.0, 4, true));
        let expected = {
            let mut data = json::Object::new();
            data.insert("one".to_owned(), Value::Number(Number::F64(1.0)));
            data.insert("two".to_owned(), Value::Number(Number::F64(2.0)));
            data.insert("three".to_owned(), Value::Number(Number::F64(3.0)));
            data.insert("four".to_owned(), Value::Number(Number::I64(4)));
            data.insert("five".to_owned(), Value::Bool(true));
            json::to_string(&data)
        };
        assert_eq!(String::from_utf8(output.to_vec()).unwrap(), expected);

        // Test with 6 elements
        let mut block = JsonDumpBlock::<(f64, f32, f32, i32, bool, ByteSliceSignal)>::default();
        let parameters = Parameters::new(&[
            "Default:one".to_owned(),
            "Default:two".to_owned(),
            "Default:three".to_owned(),
            "Default:four".to_owned(),
            "Default:five".to_owned(),
            "Utf8:six".to_owned(),
        ]);
        let output = block.process(
            &parameters,
            &ctxt,
            (1.0, 2.0, 3.0, 4, true, b"six".as_ref()),
        );
        let expected = {
            let mut data = json::Object::new();
            data.insert("one".to_owned(), Value::Number(Number::F64(1.0)));
            data.insert("two".to_owned(), Value::Number(Number::F64(2.0)));
            data.insert("three".to_owned(), Value::Number(Number::F64(3.0)));
            data.insert("four".to_owned(), Value::Number(Number::I64(4)));
            data.insert("five".to_owned(), Value::Bool(true));
            data.insert("six".to_owned(), Value::String("six".to_owned()));
            json::to_string(&data)
        };
        assert_eq!(String::from_utf8(output.to_vec()).unwrap(), expected);

        // Test with 7 elements
        let mut block = JsonDumpBlock::<(f64, f32, f32, i32, bool, ByteSliceSignal, u8)>::default();
        let parameters = Parameters::new(&[
            "Default:one".to_owned(),
            "Default:two".to_owned(),
            "Default:three".to_owned(),
            "Default:four".to_owned(),
            "Default:five".to_owned(),
            "Utf8:six".to_owned(),
            "Default:seven".to_owned(),
        ]);
        let output = block.process(
            &parameters,
            &ctxt,
            (1.0, 2.0, 3.0, 4, true, b"six".as_ref(), 7),
        );
        let expected = {
            let mut data = json::Object::new();
            data.insert("one".to_owned(), Value::Number(Number::F64(1.0)));
            data.insert("two".to_owned(), Value::Number(Number::F64(2.0)));
            data.insert("three".to_owned(), Value::Number(Number::F64(3.0)));
            data.insert("four".to_owned(), Value::Number(Number::I64(4)));
            data.insert("five".to_owned(), Value::Bool(true));
            data.insert("six".to_owned(), Value::String("six".to_owned()));
            data.insert("seven".to_owned(), Value::Number(Number::U64(7)));
            json::to_string(&data)
        };
        assert_eq!(String::from_utf8(output.to_vec()).unwrap(), expected);

        // Test with 8 elements
        let mut block =
            JsonDumpBlock::<(f64, f32, f32, i32, bool, ByteSliceSignal, u8, u16)>::default();
        let parameters = Parameters::new(&[
            "Default:one".to_owned(),
            "Default:two".to_owned(),
            "Default:three".to_owned(),
            "Default:four".to_owned(),
            "Default:five".to_owned(),
            "Utf8:six".to_owned(),
            "Default:seven".to_owned(),
            "Default:eight".to_owned(),
        ]);
        let output = block.process(
            &parameters,
            &ctxt,
            (1.0, 2.0, 3.0, 4, true, b"six".as_ref(), 7, 8),
        );
        let expected = {
            let mut data = json::Object::new();
            data.insert("one".to_owned(), Value::Number(Number::F64(1.0)));
            data.insert("two".to_owned(), Value::Number(Number::F64(2.0)));
            data.insert("three".to_owned(), Value::Number(Number::F64(3.0)));
            data.insert("four".to_owned(), Value::Number(Number::I64(4)));
            data.insert("five".to_owned(), Value::Bool(true));
            data.insert("six".to_owned(), Value::String("six".to_owned()));
            data.insert("seven".to_owned(), Value::Number(Number::U64(7)));
            data.insert("eight".to_owned(), Value::Number(Number::U64(8)));
            json::to_string(&data)
        };
        assert_eq!(String::from_utf8(output.to_vec()).unwrap(), expected);
    }

    #[test]
    fn test_matrix_serialization() {
        let ctxt = StubContext::default();

        // Create a simple 2x2 matrix
        let matrix = Matrix {
            data: [[1.0, 3.0], [2.0, 4.0]],
        };

        // Test without labels
        let mut block = JsonDumpBlock::<Matrix<2, 2, f64>>::default();
        let parameters = Parameters::new(&[]);
        let output = block.process(&parameters, &ctxt, &matrix);
        let expected = {
            let mut data = json::Array::new();
            let mut row1 = json::Array::new();
            row1.push(Value::Number(Number::F64(1.0)));
            row1.push(Value::Number(Number::F64(2.0)));
            let mut row2 = json::Array::new();
            row2.push(Value::Number(Number::F64(3.0)));
            row2.push(Value::Number(Number::F64(4.0)));
            data.push(Value::Array(row1));
            data.push(Value::Array(row2));
            json::to_string(&data)
        };
        assert_eq!(String::from_utf8(output.to_vec()).unwrap(), expected);

        // Test with label
        let parameters = Parameters::new(&["Default:matrix".to_owned()]);
        let output = block.process(&parameters, &ctxt, &matrix);
        let expected = {
            let mut outer_data = json::Object::new();
            let mut data = json::Array::new();
            let mut row1 = json::Array::new();
            row1.push(Value::Number(Number::F64(1.0)));
            row1.push(Value::Number(Number::F64(2.0)));
            let mut row2 = json::Array::new();
            row2.push(Value::Number(Number::F64(3.0)));
            row2.push(Value::Number(Number::F64(4.0)));
            data.push(Value::Array(row1));
            data.push(Value::Array(row2));
            outer_data.insert("matrix".to_owned(), Value::Array(data));
            json::to_string(&outer_data)
        };
        assert_eq!(String::from_utf8(output.to_vec()).unwrap(), expected);

        // Test with Utf8 encoding
        let parameters = Parameters::new(&["Utf8:matrix".to_owned()]);
        let output = block.process(&parameters, &ctxt, &matrix);
        let expected = {
            let mut outer_data = json::Object::new();
            let mut data = json::Array::new();
            let mut row1 = json::Array::new();
            row1.push(Value::Number(Number::F64(1.0)));
            row1.push(Value::Number(Number::F64(2.0)));
            let mut row2 = json::Array::new();
            row2.push(Value::Number(Number::F64(3.0)));
            row2.push(Value::Number(Number::F64(4.0)));
            data.push(Value::Array(row1));
            data.push(Value::Array(row2));
            let matrix_str = json::to_string(&data);
            outer_data.insert("matrix".to_owned(), Value::String(matrix_str));
            json::to_string(&outer_data)
        };
        assert_eq!(String::from_utf8(output.to_vec()).unwrap(), expected);
    }

    #[test]
    fn test_byte_slice_serialization() {
        let ctxt = StubContext::default();

        // Test byte slice with Utf8 encoding
        let mut block = JsonDumpBlock::<ByteSliceSignal>::default();
        let parameters = Parameters::new(&["Utf8:text".to_owned()]);
        let output = block.process(&parameters, &ctxt, b"hello".as_ref());
        let expected = {
            let mut data = json::Object::new();
            data.insert("text".to_owned(), Value::String("hello".to_owned()));
            json::to_string(&data)
        };
        assert_eq!(String::from_utf8(output.to_vec()).unwrap(), expected);

        // Test byte slice with Default encoding
        let parameters = Parameters::new(&["Default:bytes".to_owned()]);
        let output = block.process(&parameters, &ctxt, b"abc".as_ref());
        let expected = {
            let mut data = json::Object::new();
            let bytes_array = b"abc"
                .iter()
                .map(|b| Value::Number(Number::U64(u64::from(*b))));
            let bytes_array = Value::Array(json::Array::from_iter(bytes_array));
            data.insert("bytes".to_owned(), bytes_array);
            json::to_string(&data)
        };
        assert_eq!(String::from_utf8(output.to_vec()).unwrap(), expected);

        // Test non-UTF8 byte slice with Utf8 encoding (should produce empty string)
        let invalid_utf8 = &[0xFF, 0xFE, 0xFD];
        let parameters = Parameters::new(&["Utf8:text".to_owned()]);
        let output = block.process(&parameters, &ctxt, invalid_utf8.as_ref());
        let expected = {
            let mut data = json::Object::new();
            data.insert("text".to_owned(), Value::String("".to_owned()));
            json::to_string(&data)
        };
        assert_eq!(String::from_utf8(output.to_vec()).unwrap(), expected);
    }

    #[test]
    fn test_tuple_combinations() {
        let ctxt = StubContext::default();

        // Test 3-tuple
        let mut block = JsonDumpBlock::<(i32, f64, ByteSliceSignal)>::default();
        let parameters = Parameters::new(&[
            "Default:id".to_owned(),
            "Default:value".to_owned(),
            "Utf8:message".to_owned(),
        ]);
        let output = block.process(&parameters, &ctxt, (42, 3.1, b"test".as_ref()));
        let expected = {
            let mut data = json::Object::new();
            data.insert("id".to_owned(), Value::Number(Number::I64(42)));
            data.insert("value".to_owned(), Value::Number(Number::F64(3.1)));
            data.insert("message".to_owned(), Value::String("test".to_owned()));
            json::to_string(&data)
        };
        assert_eq!(String::from_utf8(output.to_vec()).unwrap(), expected);

        // Test 4-tuple
        let mut block = JsonDumpBlock::<(i32, f64, ByteSliceSignal, bool)>::default();
        let parameters = Parameters::new(&[
            "Default:id".to_owned(),
            "Utf8:value".to_owned(),
            "Default:data".to_owned(),
            "Default:flag".to_owned(),
        ]);
        let output = block.process(&parameters, &ctxt, (42, 3.1, b"test".as_ref(), true));
        let expected = {
            let mut data = json::Object::new();
            data.insert("id".to_owned(), Value::Number(Number::I64(42)));
            data.insert("value".to_owned(), Value::String("3.1".to_owned()));
            let bytes_array = b"test"
                .iter()
                .map(|b| Value::Number(Number::U64(u64::from(*b))));
            let bytes_array = Value::Array(json::Array::from_iter(bytes_array));
            data.insert("data".to_owned(), bytes_array);
            data.insert("flag".to_owned(), Value::Bool(true));
            json::to_string(&data)
        };
        assert_eq!(String::from_utf8(output.to_vec()).unwrap(), expected);
    }

    #[test]
    fn test_parsing_output_spec() {
        // Test valid specs
        let specs = vec!["Default:value".to_owned(), "Utf8:text".to_owned()];
        let params = Parameters::new(&specs);
        assert_eq!(params.encoding_spec.len(), 2);
        assert_eq!(params.encoding_spec[0].0, EncodingType::Default);
        assert_eq!(params.encoding_spec[0].1, "value");
        assert_eq!(params.encoding_spec[1].0, EncodingType::Utf8);
        assert_eq!(params.encoding_spec[1].1, "text");
    }

    #[test]
    #[should_panic(expected = "Invalid output data format")]
    fn test_invalid_output_spec() {
        // Test invalid spec format (missing colon)
        let specs = vec!["InvalidSpec".to_owned()];
        let _params = Parameters::new(&specs);
    }

    #[test]
    #[should_panic(expected = "assertion failed")]
    fn test_parameter_count_mismatch() {
        let ctxt = StubContext::default();
        // Test with more labels than inputs
        let mut block = JsonDumpBlock::<(f64, f32)>::default();
        let parameters = Parameters::new(&[
            "Default:foo".to_owned(),
            "Default:bar".to_owned(),
            "Default:extra".to_owned(),
        ]);
        block.process(&parameters, &ctxt, (1.0, 2.0));
    }
}
