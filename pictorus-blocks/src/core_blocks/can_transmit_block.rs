extern crate alloc;
use crate::traits::{Float, Scalar};
use alloc::vec::Vec;
use block_data::BlockData as OldBlockData;
use pictorus_traits::{ByteSliceSignal, Pass, PassBy, ProcessBlock};

// Ideally this would not have to return a new vec, but that added a lot of complexity
// in the generated code. Possible we can simplify once we're generating our own CAN structs
type TxCallback<S, C> = fn(&[S], &mut C) -> Result<Vec<u8>, ()>;

/// Parameters for the CanTransmitBlock
#[doc(hidden)]
pub struct Parameters {
    // CAN frame ID
    pub frame_id: embedded_can::Id,
}

impl Parameters {
    pub fn new(frame_id: embedded_can::Id) -> Self {
        Parameters { frame_id }
    }
}

/// Converts signals (as defined by the associated DBC message) to a CAN data frame.
pub struct CanTransmitBlock<
    // The type of the input signal (e.g., f32, f64). Currently either f32 or f64.
    S: Float,
    // The struct type for encoding and decoding the CAN data frame.
    C,
    // A tuple of input data (1 to 8 values of type S) to convert to an 8 byte CAN data payload.
    I: Pass,
> {
    pub data: Vec<OldBlockData>,
    byte_buffer: Vec<u8>,
    _phantom: core::marker::PhantomData<I>,
    tx_cb: TxCallback<S, C>,
    msg: C,
}

impl<S: Float, C, I: Pass> Default for CanTransmitBlock<S, C, I> {
    fn default() -> Self {
        panic!("CanTransmitBlock must be initialized using the ::new method");
    }
}

impl<S: Float, C, I: Pass> CanTransmitBlock<S, C, I> {
    pub fn new(tx_cb: TxCallback<S, C>, msg: C) -> Self {
        CanTransmitBlock {
            data: Vec::new(),
            _phantom: core::marker::PhantomData,
            tx_cb,
            msg,
            byte_buffer: Vec::new(),
        }
    }
}

impl<S: Float + Scalar, C, I> ProcessBlock for CanTransmitBlock<S, C, I>
where
    I: Pass + ToVec<S>,
{
    type Inputs = I;

    type Output = ByteSliceSignal;

    type Parameters = Parameters;

    fn process<'b>(
        &'b mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) -> pictorus_traits::PassBy<'b, Self::Output> {
        let mut tmp = Vec::<S>::new();
        I::to_vec(inputs, &mut tmp);

        self.byte_buffer = if let Ok(data) = (self.tx_cb)(tmp.as_slice(), &mut self.msg) {
            data
        } else {
            log::warn!("Failed to encode message data");
            Vec::new()
        };

        &self.byte_buffer
    }
}

// Trait to merge a tuple into a Vec
pub trait ToVec<S: Float>: Pass {
    fn to_vec(input: PassBy<Self>, dest: &mut Vec<S>);
}

impl<S: Float> ToVec<S> for S {
    fn to_vec(input: PassBy<Self>, dest: &mut Vec<S>) {
        dest.push(input);
    }
}

impl<S: Float> ToVec<S> for (S, S) {
    fn to_vec(input: PassBy<Self>, dest: &mut Vec<S>) {
        dest.push(input.0);
        dest.push(input.1);
    }
}

impl<S: Float> ToVec<S> for (S, S, S) {
    fn to_vec(input: PassBy<Self>, dest: &mut Vec<S>) {
        dest.push(input.0);
        dest.push(input.1);
        dest.push(input.2);
    }
}

impl<S: Float> ToVec<S> for (S, S, S, S) {
    fn to_vec(input: PassBy<Self>, dest: &mut Vec<S>) {
        dest.push(input.0);
        dest.push(input.1);
        dest.push(input.2);
        dest.push(input.3);
    }
}

impl<S: Float> ToVec<S> for (S, S, S, S, S) {
    fn to_vec(input: PassBy<Self>, dest: &mut Vec<S>) {
        dest.push(input.0);
        dest.push(input.1);
        dest.push(input.2);
        dest.push(input.3);
        dest.push(input.4);
    }
}

impl<S: Float> ToVec<S> for (S, S, S, S, S, S) {
    fn to_vec(input: PassBy<Self>, dest: &mut Vec<S>) {
        dest.push(input.0);
        dest.push(input.1);
        dest.push(input.2);
        dest.push(input.3);
        dest.push(input.4);
        dest.push(input.5);
    }
}

impl<S: Float> ToVec<S> for (S, S, S, S, S, S, S) {
    fn to_vec(input: PassBy<Self>, dest: &mut Vec<S>) {
        dest.push(input.0);
        dest.push(input.1);
        dest.push(input.2);
        dest.push(input.3);
        dest.push(input.4);
        dest.push(input.5);
        dest.push(input.6);
    }
}

impl<S: Float> ToVec<S> for (S, S, S, S, S, S, S, S) {
    fn to_vec(input: PassBy<Self>, dest: &mut Vec<S>) {
        dest.push(input.0);
        dest.push(input.1);
        dest.push(input.2);
        dest.push(input.3);
        dest.push(input.4);
        dest.push(input.5);
        dest.push(input.6);
        dest.push(input.7);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use crate::CanTransmitBlockParams as Parameters;

    extern crate alloc;
    use alloc::vec;
    use embedded_can::StandardId;

    #[test]
    fn test_can_transmit_block_scalar() {
        // Test a single CAN input signal

        let id = embedded_can::Id::Standard(StandardId::new(0x123).expect("Could not create ID"));
        let context = StubContext::default();
        let parameters = Parameters::new(id);

        struct StubCanParser {
            raw: [u8; 8],
        }

        impl StubCanParser {
            pub fn new() -> Self {
                StubCanParser { raw: [0; 8] }
            }

            pub fn set_byte_0(&mut self, data: f64) {
                self.raw[0] = data as u8;
            }
        }

        pub fn stub_can_parser_callback(
            data: &[f64],
            msg: &mut StubCanParser,
        ) -> Result<Vec<u8>, ()> {
            #[allow(clippy::len_zero)]
            if data.len() > 0 {
                msg.set_byte_0(data[0]);
            }
            Ok(msg.raw.to_vec())
        }

        let mut block = CanTransmitBlock::<f64, StubCanParser, f64>::new(
            stub_can_parser_callback,
            StubCanParser::new(),
        );
        let output = block.process(&parameters, &context, 42.0);
        assert_eq!(output, vec![42, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_can_transmit_block_tuple_2() {
        // Test a CAN tuple signal with byte and a "boolean"

        let id = embedded_can::Id::Standard(StandardId::new(0x123).expect("Could not create ID"));
        let context = StubContext::default();
        let parameters = Parameters::new(id);

        struct StubCanParser {
            raw: [u8; 8],
        }

        impl StubCanParser {
            pub fn new() -> Self {
                StubCanParser { raw: [0; 8] }
            }

            pub fn set_byte_0(&mut self, data: f64) {
                self.raw[0] = data as u8;
            }

            pub fn set_bit_9(&mut self, data: bool) {
                if data {
                    self.raw[1] |= 1;
                } else {
                    self.raw[1] &= 0xFE;
                }
            }
        }

        pub fn stub_can_parser_callback(
            data: &[f64],
            msg: &mut StubCanParser,
        ) -> Result<Vec<u8>, ()> {
            #[allow(clippy::len_zero)]
            if data.len() > 0 {
                msg.set_byte_0(data[0]);
            }
            if data.len() > 1 {
                msg.set_bit_9(data[1] != 0.0);
            }
            Ok(msg.raw.to_vec())
        }

        let mut block = CanTransmitBlock::<f64, StubCanParser, (f64, f64)>::new(
            stub_can_parser_callback,
            StubCanParser::new(),
        );
        let output = block.process(&parameters, &context, (42.0, 1.0));
        assert_eq!(output, vec![42, 1, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_can_transmit_block_tuple_7() {
        // Test a 7 input CAN signal

        let id = embedded_can::Id::Standard(StandardId::new(0x123).expect("Could not create ID"));
        let context = StubContext::default();
        let parameters = Parameters::new(id);

        struct StubCanParser {
            raw: [u8; 8],
        }

        impl StubCanParser {
            pub fn new() -> Self {
                StubCanParser { raw: [0; 8] }
            }

            pub fn set_byte_0(&mut self, data: f64) {
                self.raw[0] = data as u8;
            }

            pub fn set_byte_1(&mut self, data: f64) {
                self.raw[1] = data as u8;
            }

            pub fn set_byte_2(&mut self, data: f64) {
                self.raw[2] = data as u8;
            }

            pub fn set_byte_3(&mut self, data: f64) {
                self.raw[3] = data as u8;
            }

            pub fn set_byte_4(&mut self, data: f64) {
                self.raw[4] = data as u8;
            }

            pub fn set_byte_5(&mut self, data: f64) {
                self.raw[5] = data as u8;
            }

            pub fn set_byte_6(&mut self, data: f64) {
                self.raw[6] = data as u8;
            }
        }

        pub fn stub_can_parser_callback(
            data: &[f64],
            msg: &mut StubCanParser,
        ) -> Result<Vec<u8>, ()> {
            #[allow(clippy::len_zero)]
            if data.len() > 0 {
                msg.set_byte_0(data[0]);
            }
            if data.len() > 1 {
                msg.set_byte_1(data[1]);
            }
            if data.len() > 2 {
                msg.set_byte_2(data[2]);
            }
            if data.len() > 3 {
                msg.set_byte_3(data[3]);
            }
            if data.len() > 4 {
                msg.set_byte_4(data[4]);
            }
            if data.len() > 5 {
                msg.set_byte_5(data[5]);
            }
            if data.len() > 6 {
                msg.set_byte_6(data[6]);
            }
            Ok(msg.raw.to_vec())
        }

        let mut block =
            CanTransmitBlock::<f64, StubCanParser, (f64, f64, f64, f64, f64, f64, f64)>::new(
                stub_can_parser_callback,
                StubCanParser::new(),
            );
        let output = block.process(&parameters, &context, (10., 9., 8., 7., 6., 5., 4.));
        assert_eq!(output, vec![10, 9, 8, 7, 6, 5, 4, 0]);
    }
}
