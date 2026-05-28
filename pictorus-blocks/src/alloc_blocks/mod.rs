//! Blocks that depend on the `alloc` crate (heap allocation).
//!
//! These blocks are only compiled when the `alloc` feature is enabled.

mod bytes_join_block;
pub use bytes_join_block::BytesJoinBlock;

mod bytes_pack_block;
pub use bytes_pack_block::BytesPackBlock;

mod bytes_split_block;
pub use bytes_split_block::BytesSplitBlock;

mod bytes_unpack_block;
pub use bytes_unpack_block::BytesUnpackBlock;

mod can_transmit_block;
pub use can_transmit_block::CanTransmitBlock;
#[doc(hidden)]
pub use can_transmit_block::Parameters as CanTransmitBlockParams;

mod i2c_input_block;
pub use i2c_input_block::I2cInputBlock;
#[doc(hidden)]
pub use i2c_input_block::Parameters as I2cInputBlockParams;

mod i2c_output_block;
pub use i2c_output_block::I2cOutputBlock;
#[doc(hidden)]
pub use i2c_output_block::Parameters as I2cOutputBlockParams;

mod json_dump_block;
pub use json_dump_block::JsonDumpBlock;

mod json_load_block;
pub use json_load_block::JsonLoadBlock;

mod serial_receive_block;
#[doc(hidden)]
pub use serial_receive_block::Parameters as SerialReceiveBlockParams;
pub use serial_receive_block::SerialReceiveBlock;

mod serial_transmit_block;
#[doc(hidden)]
pub use serial_transmit_block::Parameters as SerialTransmitBlockParams;
pub use serial_transmit_block::SerialTransmitBlock;

mod spi_receive_block;
#[doc(hidden)]
pub use spi_receive_block::Parameters as SpiReceiveBlockParams;
pub use spi_receive_block::SpiReceiveBlock;

mod string_format_block;
pub use string_format_block::StringFormatBlock;

mod switch_block;
pub use switch_block::SwitchBlock;
