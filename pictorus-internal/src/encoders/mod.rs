use serde::Serialize;

pub mod postcard_encoder;

pub trait PictorusEncoder {
    fn encode<const N: usize>(&mut self, data: &impl Serialize) -> heapless::Vec<u8, N>;
}
