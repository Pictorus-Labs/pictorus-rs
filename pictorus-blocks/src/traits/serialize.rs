extern crate alloc;
use super::Scalar;
use alloc::{string::String, vec::Vec};
use miniserde::{
    json::{self, Array, Number, Value},
    ser::Fragment,
};
use pictorus_traits::{ByteSliceSignal, Matrix, Pass, PassBy};

/// A trait for serializing into a JSON value.
/// This trait is used to convert various data types into a JSON representation.
/// It can provide the internal [`miniserde::json::Value`] of the data or
/// serialize it into a byte array.
///
/// Optionally supports custom format options for serialization.
pub trait Serialize: Pass {
    /// The options for formatting the serialized data
    /// This is used to customize the serialization process for a specific type if required.
    type FormatOptions: Default;

    /// Returns self as a JSON value.
    fn as_json_value(input: PassBy<Self>, options: Self::FormatOptions) -> json::Value;

    /// Converts the value into a byte array, this is just a wrapper around
    /// `as_json_value` that serializes the resultant JSON value as a string and
    /// then returns that string as a byte array.
    fn to_bytes(value: PassBy<Self>, options: Self::FormatOptions) -> Vec<u8> {
        json::to_string(&Self::as_json_value(value, options)).into_bytes()
    }

    /// Converts the value into a byte array using default options.
    /// This is a convenience method that uses the default format options for
    /// serialization.
    fn to_bytes_default(value: PassBy<Self>) -> Vec<u8> {
        Self::to_bytes(value, Self::FormatOptions::default())
    }
}

impl<S: Scalar + miniserde::Serialize> Serialize for S {
    type FormatOptions = ();

    fn as_json_value(input: PassBy<Self>, _: ()) -> json::Value {
        let fragment = input.begin();
        match fragment {
            Fragment::F64(v) => Value::Number(Number::F64(v)),
            Fragment::I64(v) => Value::Number(Number::I64(v)),
            Fragment::U64(v) => Value::Number(Number::U64(v)),
            Fragment::Bool(v) => Value::Bool(v),
            _ => panic!("The Scalar bound means this should never happen"),
        }
    }
}

impl<const NROWS: usize, const NCOLS: usize, S: Scalar + Serialize> Serialize
    for Matrix<NROWS, NCOLS, S>
where
    S: Serialize<FormatOptions = ()>,
{
    type FormatOptions = ();

    fn as_json_value(input: PassBy<Self>, _: ()) -> json::Value {
        let mut data = Array::new();
        for row in 0..NROWS {
            let mut row_data = Array::new();
            for col in 0..NCOLS {
                row_data.push(S::as_json_value(input.data[col][row], ()));
            }
            data.push(Value::Array(row_data));
        }
        Value::Array(data)
    }
}
/// Options for formatting a [`ByteSliceSignal`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ByteSliceFormat {
    /// Format as a JSON array of numbers.
    /// Example: `[104,101,108,108,111]: &[u8]` maps to a json representation of `[104,101,108,108,111]`
    Array,
    /// Format as a JSON string.
    /// Example: `[104,101,108,108,111]: &[u8]` maps to a json representation of`"hello"`
    String,
    /// Pass through the raw bytes without any formatting. This is the default option.
    /// This is only useful for calls to `to_bytes()`. Where it will literally
    /// return the bytes it was passed. If used in `as_json_value()`, it will
    /// return `Value::Null`.
    ///
    /// Example: `[104,101,108,108,111]: &[u8]` maps to the literal bytes `[104,101,108,108,111]`
    #[default]
    RawBytes,
}

impl Serialize for ByteSliceSignal {
    type FormatOptions = ByteSliceFormat;

    fn as_json_value(input: PassBy<Self>, options: Self::FormatOptions) -> json::Value {
        match options {
            ByteSliceFormat::Array => {
                let mut data = Array::new();
                for byte in input.iter().map(|byte| u64::from(*byte)) {
                    data.push(Value::Number(Number::U64(byte)));
                }
                Value::Array(data)
            }
            ByteSliceFormat::String => {
                Value::String(String::from_utf8(input.to_vec()).unwrap_or_default())
            }
            ByteSliceFormat::RawBytes => Value::Null, // This Option is for passing through raw bytes in `to_bytes()`
        }
    }

    fn to_bytes(value: PassBy<Self>, options: Self::FormatOptions) -> Vec<u8> {
        match options {
            ByteSliceFormat::RawBytes => value.to_vec(),
            _ => json::to_string(&Self::as_json_value(value, options)).into_bytes(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_scalar_f64() {
        let scalar = 42.42;
        let json_val = f64::as_json_value(scalar, ());
        assert_eq!(
            json::to_string(&json_val),
            json::to_string(&Value::Number(Number::F64(42.42)))
        );

        let bytes = f64::to_bytes_default(scalar);
        assert_eq!(bytes.as_slice(), [b'4', b'2', b'.', b'4', b'2']);
    }

    #[test]
    fn test_serialize_byte_slice_signal() {
        let byte_arr: [u8; 5] = [104, 101, 108, 108, 111]; // bytes for "hello"
        let json_val = ByteSliceSignal::as_json_value(&byte_arr, ByteSliceFormat::Array);
        assert_eq!(json::to_string(&json_val).as_str(), "[104,101,108,108,111]");

        let bytes = ByteSliceSignal::to_bytes(&byte_arr, ByteSliceFormat::RawBytes);
        assert_eq!(bytes.as_slice(), [104, 101, 108, 108, 111]);

        let bytes = ByteSliceSignal::to_bytes(&byte_arr, ByteSliceFormat::String);
        assert_eq!(bytes.as_slice(), b"\"hello\"");
    }

    #[test]
    fn test_serialize_matrix() {
        let matrix = Matrix {
            data: [[1.0, 4.0], [2.0, 5.0], [3.0, 6.0]],
        };
        let json_val = Matrix::<2, 3, f64>::as_json_value(&matrix, ());
        assert_eq!(json::to_string(&json_val), "[[1.0,2.0,3.0],[4.0,5.0,6.0]]");

        let bytes = Matrix::<2, 3, f64>::to_bytes(&matrix, ());
        assert_eq!(bytes.as_slice(), "[[1.0,2.0,3.0],[4.0,5.0,6.0]]".as_bytes());
    }
}
