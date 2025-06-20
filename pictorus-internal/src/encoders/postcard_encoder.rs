use crate::encoders::PictorusEncoder;
use serde::Serialize;

pub struct PostcardEncoderCOBS {}

impl PictorusEncoder for PostcardEncoderCOBS {
    fn encode<const N: usize>(&mut self, data: &impl Serialize) -> heapless::Vec<u8, N> {
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
    use serde::Serialize;

    #[derive(Serialize)]
    struct LogData {
        app_time: f64,
    }

    #[test]
    fn test_postcard_encoder() {
        let log_data = LogData { app_time: 1.0 };

        let mut encoder = PostcardEncoderCOBS {};
        let encoded = encoder.encode::<64>(&log_data);

        // COBS encoding removes zero bytes and adds a sentinel byte, so the length of an 8 byte float
        // with some 0's in it will be 10 bytes.
        assert!(encoded.len() == 10);
    }
}
