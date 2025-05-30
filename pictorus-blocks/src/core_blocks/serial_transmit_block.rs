extern crate alloc;
use alloc::vec::Vec;
use log::debug;
use pictorus_block_data::BlockData as OldBlockData;
use pictorus_traits::{ByteSliceSignal, Context, Pass, PassBy, ProcessBlock};

use crate::byte_data::{parse_string_to_bytes, BUFF_SIZE_BYTES};
use crate::traits::Serialize;

/// Parameters for the SerialTransmitBlock
#[doc(hidden)]
pub struct Parameters {
    /// Start delimiter for the serial transmission, will be prepended to the data
    start_delimiter: Vec<u8>,
    /// End delimiter for the serial transmission, will be appended to the data
    end_delimiter: Vec<u8>,
}

impl Parameters {
    pub fn new(start_delimiter: &str, end_delimiter: &str) -> Self {
        Parameters {
            start_delimiter: parse_string_to_bytes(start_delimiter),
            end_delimiter: parse_string_to_bytes(end_delimiter),
        }
    }
}

/// Encodes data for transmission over the serial interface, by
/// prepending the start delimiter and appending the end delimiter to the data.
///
/// Data is output as a ByteSliceSignal.
pub struct SerialTransmitBlock<T: Serialize + Pass> {
    pub data: OldBlockData,
    buffer: Vec<u8>,
    phantom: core::marker::PhantomData<T>,
}

impl<T> Default for SerialTransmitBlock<T>
where
    T: Serialize + Pass,
{
    fn default() -> Self {
        SerialTransmitBlock {
            data: OldBlockData::from_bytes(&[]),
            buffer: Vec::with_capacity(BUFF_SIZE_BYTES),
            phantom: core::marker::PhantomData,
        }
    }
}

impl<T> ProcessBlock for SerialTransmitBlock<T>
where
    T: Serialize + Pass,
{
    type Parameters = Parameters;
    type Inputs = T;
    type Output = ByteSliceSignal;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        _context: &dyn Context,
        input: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        let write_val = [
            parameters.start_delimiter.as_slice(),
            T::to_bytes_default(input).as_slice(),
            parameters.end_delimiter.as_slice(),
        ]
        .concat();
        debug!("Transmitting value: {:?}", &write_val);
        self.data = OldBlockData::from_bytes(&write_val);
        self.buffer = write_val;
        &self.buffer
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::StubContext;
    use pictorus_traits::Matrix;

    use super::*;

    #[test]
    fn test_write_byteslicesignal_no_delimiters() {
        let context = StubContext::default();
        let parameters = Parameters::new("", "");
        let mut block = SerialTransmitBlock::<ByteSliceSignal>::default();

        let expected = "42".as_bytes();
        let to_serial_peripheral = block.process(&parameters, &context, expected);
        assert_eq!(to_serial_peripheral, expected);
    }

    #[test]
    fn test_write_byteslicesignal_delimited_data() {
        let context = StubContext::default();
        let parameters = Parameters::new("$GPGSA,", "\r\n");
        let mut block = SerialTransmitBlock::<ByteSliceSignal>::default();

        let expected = "$GPGSA,123\r\n".as_bytes();

        let to_serial_peripheral = block.process(&parameters, &context, b"123");
        assert_eq!(to_serial_peripheral, expected);
    }

    #[test]
    fn test_write_matrix() {
        let context = StubContext::default();
        let parameters = Parameters::new("$GPGSA,", "\r\n");
        let mut block = SerialTransmitBlock::<Matrix<2, 3, f64>>::default();

        // Second input is a matrix
        // Our matrices our col-major, so we would expect an input matrix of
        // [[1.0, 4.0], [2.0, 5.0], [3.0, 6.0]] to be serialized as
        // [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]
        let input = Matrix {
            data: [[1.0, 4.0], [2.0, 5.0], [3.0, 6.0]],
        };

        let expected = "$GPGSA,[[1.0,2.0,3.0],[4.0,5.0,6.0]]\r\n".as_bytes();

        let to_serial_peripheral = block.process(&parameters, &context, &input);
        assert_eq!(to_serial_peripheral, expected);
    }

    #[test]
    fn test_write_scalar() {
        let context = StubContext::default();
        let parameters = Parameters::new("$GPGSA,", "\r\n");
        let mut block = SerialTransmitBlock::<f64>::default();

        let expected = "$GPGSA,8675.309\r\n".as_bytes();

        let to_serial_peripheral = block.process(&parameters, &context, 8675.309);
        assert_eq!(to_serial_peripheral, expected);
    }

    #[test]
    fn test_write_vector() {
        let context = StubContext::default();
        let parameters = Parameters::new("$GPGSA,", "\r\n");
        let mut block = SerialTransmitBlock::<Matrix<1, 3, f64>>::default();

        let expected = "$GPGSA,[[1.2,3.4,5.6]]\r\n".as_bytes();

        let input = Matrix {
            data: [[1.2], [3.4], [5.6]],
        };

        let to_serial_peripheral = block.process(&parameters, &context, &input);
        assert_eq!(to_serial_peripheral, expected);
    }

    #[test]
    fn test_write_hex() {
        let context = StubContext::default();
        let parameters = Parameters::new("", "");
        let mut block = SerialTransmitBlock::<ByteSliceSignal>::default();

        let expected = "\x12\x34".as_bytes();

        let to_serial_peripheral = block.process(&parameters, &context, b"\x12\x34");
        assert_eq!(to_serial_peripheral, expected);
    }
}
