use alloc::vec::Vec;
use serde::Serialize;

pub mod postcard_encoder;

pub trait PictorusEncoder {
    fn encode<'a>(&mut self, data: &impl Serialize, buffer: &mut Vec<u8>);
}
