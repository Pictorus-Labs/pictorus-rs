use crate::encoders::PictorusEncoder;
use alloc::vec::Vec;
use serde::Serialize;

pub struct PostcardEncoder {}

impl PictorusEncoder for PostcardEncoder {
    fn encode(&mut self, data: &impl Serialize, buffer: &mut Vec<u8>) {
        buffer.clear();
        match postcard::to_allocvec_cobs(data) {
            Ok(data) => {
                buffer.extend(data);
            }
            Err(_) => {} // Clear the buffer if encoding fails
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

        let mut encoder = PostcardEncoder {};
        let mut buffer = alloc::vec::Vec::<u8>::new(); // Buffer size should be large enough for the encoded data
        encoder.encode(&log_data, &mut buffer);

        // COBS encoding removes zero bytes and adds a sentinel byte, so the length of an 8 byte float
        // with some 0's in it will be 10 bytes.
        assert!(buffer.len() == 10);
    }
}
