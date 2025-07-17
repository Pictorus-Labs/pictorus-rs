use serde::Serialize;

pub struct PostcardEncoderCOBS {}

impl PostcardEncoderCOBS {
    pub fn encode<const N: usize>(&mut self, data: &impl Serialize) -> heapless::Vec<u8, N> {
        match postcard::to_vec_cobs(data) {
            Ok(encoded) => encoded,
            Err(_) => {
                log::warn!(
                    "Failed to encode data with Postcard, possibly too much data for the buffer."
                );
                heapless::Vec::<u8, N>::new()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;
    use serde::{Deserialize, Serialize};

    #[test]
    fn test_common_pictorus_log_data() {
        #[derive(Serialize, Deserialize, Debug)]
        struct LogData<'a> {
            pub timestamp: Option<u64>,
            pub state_name: Option<&'a str>,
            pub matrix: Option<[[f64; 4]; 4]>,
            pub byte_array: Option<Vec<u8>>,
        }

        let matrix = Some([
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        ]);
        let byte_array = Some(vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let state_name = Some("test_state");
        let log_data = LogData {
            timestamp: Some(1234567890),
            state_name,
            matrix,
            byte_array,
        };

        // Size the buffer:
        //   16*8 (16 f64 matrix, no length) +
        //   (8 + 1) (byte array + u8 length) +
        //   (10 + 1) (state name + u8 length) +
        //   (4) (timestamp is a u64, but should be varint encoded to a u32)
        //   (4) (4 bytes for the 4 options)
        let mut decode_buffer = [0u8; 160];

        let mut encoder = PostcardEncoderCOBS {};
        let encoded = encoder.encode::<161>(&log_data); // Allocate an extra byte for COBS encoding

        // Un-COBS-ify
        let _decode =
            cobs::decode(&encoded, &mut decode_buffer).expect("Successfully un-COBS-ified");

        let log_data_decoded: LogData =
            postcard::from_bytes(&decode_buffer).expect("Successfully decoded");

        assert!(log_data_decoded.timestamp == Some(1234567890));
        assert!(log_data_decoded.state_name == Some("test_state"));
        assert!(
            log_data_decoded.matrix
                == Some([
                    [1.0, 2.0, 3.0, 4.0],
                    [5.0, 6.0, 7.0, 8.0],
                    [9.0, 10.0, 11.0, 12.0],
                    [13.0, 14.0, 15.0, 16.0],
                ])
        );
        assert!(log_data_decoded.byte_array == Some(vec![1, 2, 3, 4, 5, 6, 7, 8]));
    }
}
