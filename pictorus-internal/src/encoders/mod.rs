use serde::Serialize;
use alloc::vec::Vec;

pub mod postcard_encoder;

pub trait PictorusEncoder {
    fn encode<'a>(&mut self, data: &impl Serialize, buffer: &mut Vec<u8>);
}