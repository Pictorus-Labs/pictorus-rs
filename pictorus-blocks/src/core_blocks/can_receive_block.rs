extern crate alloc;
use block_data::BlockData as OldBlockData;
use pictorus_traits::{ByteSliceSignal, Pass, ProcessBlock};

use crate::{stale_tracker::StaleTracker, traits::Float, IsValid};

type RxCallback<C, S> = fn(&C, &mut [S]);

/// Parameters for the CanReceiveBlock
pub struct Parameters {
    /// ID of the CAN frame
    pub frame_id: embedded_can::Id,
    /// Number of bytes in the data frame
    length: usize,
    /// Stale age in milliseconds
    stale_age_ms: f64,
}

impl Parameters {
    pub fn new(frame_id: embedded_can::Id, length: usize, stale_age_ms: f64) -> Self {
        Self {
            frame_id,
            length,
            stale_age_ms,
        }
    }
}

/// The CanReceiveBlock takes a ByteSliceSignal that represents a CAN data frame
/// and converts it into a single float or tuple of floats that map to the quantized
/// output signals as defined in the CAN DBC file.
/// Generics:
/// - N: The number of signals in the CAN frame.
/// - S: The type of the input signal (e.g., f32, f64). Currently either f32 or f64.
/// - C: The struct type for encoding and decoding the CAN data frame.
/// - I: A scalar or tuple of output data (1 to 8 values of type S) quantized based on the CAN DBC file.
pub struct CanReceiveBlock<const N: usize, S: Float, C, O: Pass + Default> {
    rx_cb: RxCallback<C, S>,
    stale_check: StaleTracker,
    _phantom: core::marker::PhantomData<O>,
    output_buffer: O,
    pub data: [OldBlockData; N],
    cache: [S; N],
    previous_stale_check_time_ms: f64,
}

impl<const N: usize, S: Float, C, O: Pass + Default> Default for CanReceiveBlock<N, S, C, O> {
    fn default() -> Self {
        panic!("CanReceiveBlock must be initialized using the ::new method");
    }
}

impl<const N: usize, S: Float, C, O: Pass + Default> CanReceiveBlock<N, S, C, O> {
    pub fn new(rx_cb: RxCallback<C, S>) -> Self {
        let data = core::array::from_fn(|_f| OldBlockData::from_scalar(0.));
        let cache = [S::zero(); N];
        CanReceiveBlock {
            rx_cb,
            stale_check: StaleTracker::from_ms(0.),
            _phantom: core::marker::PhantomData,
            data,
            output_buffer: O::default(),
            cache,
            previous_stale_check_time_ms: 0.0,
        }
    }
}

impl<const N: usize, S: Float, C: embedded_can::Frame, O: Pass + Default> ProcessBlock
    for CanReceiveBlock<N, S, C, O>
where
    O: ToTuple<S>,
    S: From<f64>,
{
    type Inputs = ByteSliceSignal;

    type Output = O;

    type Parameters = Parameters;

    fn process(
        &mut self,
        parameters: &Self::Parameters,
        context: &dyn pictorus_traits::Context,
        inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) -> pictorus_traits::PassBy<Self::Output> {
        if self.previous_stale_check_time_ms != parameters.stale_age_ms {
            self.stale_check = StaleTracker::from_ms(parameters.stale_age_ms);
            self.previous_stale_check_time_ms = parameters.stale_age_ms;
        }

        if inputs.len() == parameters.length {
            if let Some(can_decoder) = C::new(parameters.frame_id, inputs) {
                (self.rx_cb)(&can_decoder, self.cache.as_mut_slice());
                self.stale_check.mark_updated(context.time().as_secs_f64());

                for (i, val) in self.cache.iter().enumerate() {
                    self.data[i] = OldBlockData::from_scalar((*val).into());
                }
            }
        }

        // Update buffer
        self.output_buffer = O::to_tuple(&self.cache)
            .expect("parameters.signal_count is shorter than output tuple type");
        self.output_buffer.as_by()
    }
}

impl<const N: usize, S: Float, C, O: Pass + Default> IsValid for CanReceiveBlock<N, S, C, O> {
    fn is_valid(&self, app_time_s: f64) -> OldBlockData {
        self.stale_check.is_valid(app_time_s)
    }
}

/// Trait to convert a vector of floats into a tuple.
trait ToTuple<S: Float>: Sized {
    fn to_tuple(vec: &[S]) -> Result<Self, ()>;
}

impl<S: Float + Sized> ToTuple<S> for S {
    fn to_tuple(vec: &[S]) -> Result<Self, ()> {
        if vec.is_empty() {
            Err(())
        } else {
            Ok(vec[0])
        }
    }
}

impl<S: Float> ToTuple<S> for (S, S) {
    fn to_tuple(vec: &[S]) -> Result<Self, ()> {
        if vec.len() < 2 {
            Err(())
        } else {
            Ok((vec[0], vec[1]))
        }
    }
}

impl<S: Float> ToTuple<S> for (S, S, S) {
    fn to_tuple(vec: &[S]) -> Result<Self, ()> {
        if vec.len() < 3 {
            Err(())
        } else {
            Ok((vec[0], vec[1], vec[2]))
        }
    }
}

impl<S: Float> ToTuple<S> for (S, S, S, S) {
    fn to_tuple(vec: &[S]) -> Result<Self, ()> {
        if vec.len() < 4 {
            Err(())
        } else {
            Ok((vec[0], vec[1], vec[2], vec[3]))
        }
    }
}

impl<S: Float> ToTuple<S> for (S, S, S, S, S) {
    fn to_tuple(vec: &[S]) -> Result<Self, ()> {
        if vec.len() < 5 {
            Err(())
        } else {
            Ok((vec[0], vec[1], vec[2], vec[3], vec[4]))
        }
    }
}

impl<S: Float> ToTuple<S> for (S, S, S, S, S, S) {
    fn to_tuple(vec: &[S]) -> Result<Self, ()> {
        if vec.len() < 6 {
            Err(())
        } else {
            Ok((vec[0], vec[1], vec[2], vec[3], vec[4], vec[5]))
        }
    }
}

impl<S: Float> ToTuple<S> for (S, S, S, S, S, S, S) {
    fn to_tuple(vec: &[S]) -> Result<Self, ()> {
        if vec.len() < 7 {
            Err(())
        } else {
            Ok((vec[0], vec[1], vec[2], vec[3], vec[4], vec[5], vec[6]))
        }
    }
}

impl<S: Float> ToTuple<S> for (S, S, S, S, S, S, S, S) {
    fn to_tuple(vec: &[S]) -> Result<Self, ()> {
        if vec.len() < 8 {
            Err(())
        } else {
            Ok((
                vec[0], vec[1], vec[2], vec[3], vec[4], vec[5], vec[6], vec[7],
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use super::*;
    use crate::testing::StubRuntime;
    extern crate alloc;
    use embedded_can::StandardId;

    const MAX_BYTES_PER_CAN_DATA_FRAME: usize = 8;

    struct StubCanParser {
        raw: [u8; MAX_BYTES_PER_CAN_DATA_FRAME],
    }

    // Implement a bare minimum impl of the Frame trait for the stub
    impl embedded_can::Frame for StubCanParser {
        fn new(_id: impl Into<embedded_can::Id>, data: &[u8]) -> Option<Self> {
            let mut raw = [0; MAX_BYTES_PER_CAN_DATA_FRAME];
            raw.copy_from_slice(data);
            Some(StubCanParser { raw })
        }

        fn new_remote(_id: impl Into<embedded_can::Id>, _dlc: usize) -> Option<Self> {
            todo!()
        }

        fn is_extended(&self) -> bool {
            todo!()
        }

        fn is_remote_frame(&self) -> bool {
            todo!()
        }

        fn id(&self) -> embedded_can::Id {
            todo!()
        }

        fn dlc(&self) -> usize {
            todo!()
        }

        fn data(&self) -> &[u8] {
            todo!()
        }
    }

    impl StubCanParser {
        pub fn get_signal(&self, index: usize) -> f32 {
            self.raw[index] as f32
        }
    }

    fn stub_can_parser_callback(msg: &StubCanParser, data: &mut [f64]) {
        for (i, val) in data.iter_mut().enumerate() {
            *val = msg.get_signal(i).into();
        }
    }

    #[test]
    fn test_can_receive_simple_1_signal() {
        let id = embedded_can::Id::Standard(StandardId::new(0x123).expect("Could not create ID"));
        let mut runtime = StubRuntime::default();

        let parameters = Parameters::new(id, 8, 1000.0);

        let mut block =
            CanReceiveBlock::<1, f64, StubCanParser, f64>::new(stub_can_parser_callback);

        let output = block.process(&parameters, &runtime.context(), &[42, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(output, 42.);
        assert_eq!(
            block
                .is_valid(runtime.context().time.as_secs_f64())
                .scalar(),
            1.0
        );

        runtime.set_time(Duration::from_secs(2));

        // Simulate a stale message
        let output = block.process(&parameters, &runtime.context(), &[]);
        assert_eq!(output, 42.);
        assert_eq!(
            block
                .is_valid(runtime.context().time.as_secs_f64())
                .scalar(),
            0.0
        );
    }

    #[test]
    fn test_can_receive_8_signals() {
        let id = embedded_can::Id::Standard(StandardId::new(0x123).expect("Could not create ID"));
        let mut runtime = StubRuntime::default();

        let parameters = Parameters::new(id, 8, 1000.0);

        let mut block = CanReceiveBlock::<
            8,
            f64,
            StubCanParser,
            (f64, f64, f64, f64, f64, f64, f64, f64),
        >::new(stub_can_parser_callback);

        let output = block.process(&parameters, &runtime.context(), &[42, 1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(output, (42., 1., 2., 3., 4., 5., 6., 7.));
        assert_eq!(
            block
                .is_valid(runtime.context().time.as_secs_f64())
                .scalar(),
            1.0
        );

        // Simulate stale message
        runtime.set_time(Duration::from_secs(2));

        let output = block.process(&parameters, &runtime.context(), &[]);
        assert_eq!(output, (42., 1., 2., 3., 4., 5., 6., 7.));
        assert_eq!(
            block
                .is_valid(runtime.context().time.as_secs_f64())
                .scalar(),
            0.0
        );
    }

    #[test]
    fn to_tuple() {
        // Test the conversion of a vector to a tuple of the same size
        let array_8 = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let tuple_8 = <(f64, f64, f64, f64, f64, f64, f64, f64)>::to_tuple(&array_8);
        assert!(tuple_8.is_ok());
        assert!(tuple_8.unwrap() == (1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0));

        // Test the conversion of a vector smaller than requested tuple
        let array_7 = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        let tuple_8 = <(f64, f64, f64, f64, f64, f64, f64, f64)>::to_tuple(&array_7);
        assert!(tuple_8.is_err());

        // Test the conversion of a vector larger than requested tuple
        let array_2 = [1.0, 2.0];
        let tuple_1 = <f64>::to_tuple(&array_2);
        assert!(tuple_1.is_ok());
        assert!(tuple_1.unwrap() == 1.0);
    }
}
