extern crate alloc;
use alloc::vec::Vec;
use core::{cmp::min, str};

use block_data::BlockData as OldBlockData;
use log::debug;
use pictorus_traits::{ByteSliceSignal, Context, PassBy, ProcessBlock};

use crate::{
    byte_data::{
        compare_bytes, find_bytes_idx, parse_string_to_read_delimiter, rfind_all_bytes_idx,
        rfind_bytes_idx, ByteDataError, BUFF_SIZE_BYTES,
    },
    stale_tracker::StaleTracker,
    IsValid,
};

/// Parameters for the Serial Receive Block
pub struct Parameters {
    /// A tuple of values used to scan for the start delimiter
    /// - Vec<u8> - The start delimiter byte string
    /// - Vec<usize> - The number of bytes to skip after the start delimiter
    /// - usize - The total length of the start delimiter
    start_delimiter: (Vec<u8>, Vec<usize>, usize),
    /// A tuple of values used to scan for the end delimiter
    /// - Vec<u8> - The end delimiter byte string
    /// - Vec<usize> - The number of bytes to skip after the end delimiter
    /// - usize - The total length of the end delimiter
    end_delimiter: (Vec<u8>, Vec<usize>, usize),
    /// The number of bytes to read from the peripheral
    read_bytes: usize,
    /// The age in milliseconds before the data is considered stale. Stale date is still
    /// cached until new data comes in.
    stale_age_ms: f64,
}

impl Parameters {
    pub fn new(
        start_delimiter: &str,
        end_delimiter: &str,
        read_bytes: f64,
        stale_age_ms: f64,
    ) -> Self {
        let start_delimiter = parse_string_to_read_delimiter(start_delimiter);
        let start_len = start_delimiter.0.len() + start_delimiter.1.len();
        let start_delimiter = (start_delimiter.0, start_delimiter.1, start_len);

        let end_delimiter = parse_string_to_read_delimiter(end_delimiter);
        let end_len = end_delimiter.0.len() + end_delimiter.1.len();
        let end_delimiter = (end_delimiter.0, end_delimiter.1, end_len);

        Self {
            start_delimiter,
            end_delimiter,
            read_bytes: read_bytes as usize,
            stale_age_ms,
        }
    }
}

/// The Serial Receive Block is used to parse incoming serial data by configuring the
/// start / end delimiters and the number of bytes to read. When fresh data arrives, the `is_valid`
/// signal will be pulsed high for the duration of the tick. The block caches data until a new message
/// is received and parsed.
pub struct SerialReceiveBlock {
    pub data: OldBlockData,
    buffer: Vec<u8>,
    pub stale_check: StaleTracker,
    previous_stale_check_time_ms: f64,
    output: Vec<u8>,
}

impl Default for SerialReceiveBlock {
    fn default() -> Self {
        SerialReceiveBlock {
            data: OldBlockData::from_bytes(&[]),
            buffer: Vec::new(),
            stale_check: StaleTracker::from_ms(0.0),
            previous_stale_check_time_ms: 0.0,
            output: Vec::new(),
        }
    }
}

impl IsValid for SerialReceiveBlock {
    fn is_valid(&self, app_time_s: f64) -> OldBlockData {
        self.stale_check.is_valid(app_time_s)
    }
}

impl SerialReceiveBlock {
    fn try_parse_fixed_length_data(
        &self,
        data_buff: &[u8],
        start_indices: &[usize],
        parameters: &Parameters,
    ) -> Result<(usize, usize), ByteDataError> {
        let chunk_end = data_buff.len();
        for chunk_start in start_indices {
            let offset_chunk_start = chunk_start + parameters.start_delimiter.2;
            if chunk_end < offset_chunk_start {
                continue;
            }

            if chunk_end - offset_chunk_start >= parameters.read_bytes {
                let end = offset_chunk_start + parameters.read_bytes;
                return Ok((offset_chunk_start, end));
            }
        }
        Err(ByteDataError::EndDelimiterNotFound)
    }

    fn try_parse_end_delimited_data(
        &self,
        data_buff: &[u8],
        start_indices: &[usize],
        parameters: &Parameters,
    ) -> Result<(usize, usize), ByteDataError> {
        let mut chunk_end = data_buff.len();
        for chunk_start in start_indices {
            let offset_chunk_start = chunk_start + parameters.start_delimiter.2;
            if parameters.read_bytes > 0 {
                if offset_chunk_start + parameters.read_bytes + parameters.end_delimiter.2
                    > chunk_end
                {
                    continue;
                }

                let delim_start = offset_chunk_start + parameters.read_bytes;
                let delim_end = delim_start + parameters.end_delimiter.2;
                if compare_bytes(
                    &data_buff[delim_start..delim_end],
                    &parameters.end_delimiter.0,
                    &parameters.end_delimiter.1,
                ) {
                    return Ok((offset_chunk_start, delim_start));
                }
            } else if let Ok(v) = find_bytes_idx(
                &data_buff[offset_chunk_start..chunk_end],
                &parameters.end_delimiter.0,
                &parameters.end_delimiter.1,
            ) {
                return Ok((offset_chunk_start, offset_chunk_start + v));
            }

            chunk_end = *chunk_start;
        }

        debug!("No end delimiter found");
        Err(ByteDataError::EndDelimiterNotFound)
    }

    fn parse_data(&self, parameters: &Parameters) -> Result<(usize, usize), ByteDataError> {
        let start_idx;
        let end_idx;

        // Use anything in the overflow buffer plus new data
        debug!("Received value: {:?}", &self.buffer);
        debug!(
            "Start delimiter: {:?}, End delimiter: {:?}",
            &parameters.start_delimiter, &parameters.end_delimiter
        );

        if !parameters.start_delimiter.0.is_empty() {
            let start_indices = rfind_all_bytes_idx(
                &self.buffer,
                &parameters.start_delimiter.0,
                &parameters.start_delimiter.1,
            );

            if start_indices.is_empty() {
                debug!("No start delimiter found");
                return Err(ByteDataError::StartDelimiterNotFound);
            }

            if !parameters.end_delimiter.0.is_empty() {
                (start_idx, end_idx) =
                    self.try_parse_end_delimited_data(&self.buffer, &start_indices, parameters)?;
            } else if parameters.read_bytes > 0 {
                (start_idx, end_idx) =
                    self.try_parse_fixed_length_data(&self.buffer, &start_indices, parameters)?;
            } else {
                end_idx = self.buffer.len();
                start_idx = start_indices[0] + parameters.start_delimiter.2;
            }
        } else if !parameters.end_delimiter.0.is_empty() {
            end_idx = rfind_bytes_idx(
                &self.buffer,
                &parameters.end_delimiter.0,
                &parameters.end_delimiter.1,
            )?;
            if parameters.read_bytes > 0 {
                if end_idx < parameters.read_bytes {
                    debug!("Not enough bytes to read");
                    return Err(ByteDataError::InsufficientData);
                }

                start_idx = end_idx - parameters.read_bytes;
            } else {
                start_idx = rfind_bytes_idx(
                    &self.buffer[..end_idx],
                    &parameters.end_delimiter.0,
                    &parameters.end_delimiter.1,
                )
                .map(|idx| idx + parameters.end_delimiter.2)
                .unwrap_or(0);
            }
        } else {
            start_idx = 0;
            end_idx = self.buffer.len();
        }

        Ok((start_idx, end_idx))
    }
}

// Serial Receive Block is essentially a protocol parser now. Serial data
// is read in the protocols now.
impl ProcessBlock for SerialReceiveBlock {
    type Parameters = Parameters;
    type Inputs = ByteSliceSignal;
    type Output = ByteSliceSignal;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        context: &dyn Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        if self.previous_stale_check_time_ms != parameters.stale_age_ms {
            self.stale_check = StaleTracker::from_ms(parameters.stale_age_ms);
            self.previous_stale_check_time_ms = parameters.stale_age_ms;
        }

        // Inputs is a Vec<u8> copying into a Vec<u8>
        self.buffer.extend_from_slice(inputs);

        if let Ok((start_idx, end_idx)) = self.parse_data(parameters) {
            let val = &self.buffer[start_idx..end_idx];
            debug!("Parsed value: {:?}", val);
            if start_idx != 0 {
                debug!("Discarding {} bytes", start_idx);
            }
            self.data.set_bytes(val);
            self.output = val.to_vec();

            // TODO: Drain is coming to heapless vec soon! - https://github.com/rust-embedded/heapless/pull/444
            self.buffer
                .drain(..(min(end_idx + parameters.end_delimiter.2, self.buffer.len())));

            self.stale_check.mark_updated(context.time().as_secs_f64());
        } else if self.buffer.len() >= BUFF_SIZE_BYTES * 2 {
            self.buffer.clear();
            self.buffer.extend_from_slice(inputs);
            debug!("Read too many bytes without a valid message. Clearing buffer",);
        }

        &self.output
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use super::*;
    use crate::testing::{StubContext, StubRuntime};

    #[test]
    fn test_serial_receive_block() {
        let context = StubContext::default();
        let mut block = SerialReceiveBlock::default();
        let parameters = Parameters::new("$", "\r\n", 0.0, 1000.0);

        // Test with a valid message
        let input_data = b"$Hello World\r\n";
        let result = block.process(&parameters, &context, input_data);
        assert_eq!(result, b"Hello World");
    }

    #[test]
    fn test_serial_receive_block_lots_of_nothing_then_data() {
        let context = StubContext::default();
        let mut block = SerialReceiveBlock::default();
        let parameters = Parameters::new("STX", "ETX", 0.0, 1000.0);

        // Test with stale data
        let input_data_1 = [0; 1024]; // BUFF_SIZE_BYTES is 1024

        let result = block.process(&parameters, &context, &input_data_1);
        assert_eq!(result, b"");
        assert_eq!(block.buffer.len(), 1024);

        let input_data_2 = [0; 1023]; // BUFF_SIZE_BYTES is 1023
        let result = block.process(&parameters, &context, &input_data_2); // Buffer resets at buffer >= 2048
        assert_eq!(result, b"");
        assert_eq!(block.buffer.len(), 2047);

        let input_data_3 = b"Still no delimiter";
        let result = block.process(&parameters, &context, input_data_3); // Buffer resets at buffer >= 2048
        assert_eq!(result, b"");
        assert_eq!(block.buffer.len(), b"Still no delimiter".len());

        let input_data_delimited = b"STXHelloWorldETX"; // Data with delimiter
        let result = block.process(&parameters, &context, input_data_delimited);
        assert_eq!(result, b"HelloWorld");
        assert_eq!(block.buffer.len(), 0);
    }

    #[test]
    fn test_serial_receive_block_stale_check() {
        let mut block = SerialReceiveBlock::default();
        let parameters = Parameters::new("$", "\r\n", 0.0, 1000.0);

        let mut runtime = StubRuntime::default();

        // Test with a valid message
        let input_data = b"$Hello World\r\n";
        block.process(&parameters, &runtime.context(), input_data);
        assert_eq!(
            block
                .stale_check
                .is_valid(runtime.context().time().as_secs_f64()),
            OldBlockData::from_scalar(1.0)
        );

        for i in 1..11 {
            // 100ms to 1s
            runtime.set_time(Duration::from_millis(i * 100)); // 10 Hz runtime (100ms per tick)
            block.process(&parameters, &runtime.context(), &[]);
            assert_eq!(
                block
                    .stale_check
                    .is_valid(runtime.context().time().as_secs_f64()),
                OldBlockData::from_scalar(1.0)
            );
        }

        // Stale
        runtime.set_time(Duration::from_millis(11 * 100)); // 1.1s
        block.process(&parameters, &runtime.context(), &[]);
        assert_eq!(
            block
                .stale_check
                .is_valid(runtime.context().time().as_secs_f64()),
            OldBlockData::from_scalar(0.0)
        );
    }
}
