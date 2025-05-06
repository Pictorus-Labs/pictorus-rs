extern crate alloc;
use crate::byte_data::{find_all_bytes_idx, parse_string_to_read_delimiter};
use crate::traits::{DefaultStorage, Scalar};
use crate::IsValid;
use alloc::borrow::ToOwned;
use alloc::{string::String, vec, vec::Vec};
use block_data::{BlockData as OldBlockData, FromPass};
use core::time::Duration;
use pictorus_traits::{ByteSliceSignal, Pass, PassBy, ProcessBlock};

/// The Bytes Split Block accepts as input a stream of bytes. The Pictorus UI allows users to
/// specify up to 7 outputs that they want to parse out of the input bytes. The input parsing consists of
/// splitting the input bytes based on a specified delimiter and then each output is mapped to a specific
/// index of the split byte chunks. If parsing fails the output signals will be unchanged on this specific timestep.
/// In addition to the specified output signals this block outputs a boolean "is_valid" signal. Valid in this context
/// is determined by the time since the last successful parse. If the time since the last successful parse is
/// greater than the stale age, the block will output false for the "is_valid" output.
pub struct BytesSplitBlock<T: Apply> {
    pub data: Vec<OldBlockData>,
    buffer: Option<T::Storage>,
    last_valid_time: Option<Duration>,
}

/// Parameters for the Bytes Split Block
pub struct Parameters {
    /// The delimiter used to split the input bytes, can accept hex literals and wild cards, see the [web docs](https://www.docs.pictor.us/block_reference/bytes_split_block.html)
    delimiter: String,
    /// After spliting the input by delimiter this specifies which of the split chunks is used to parse each output
    desired_output_idx: Vec<usize>,
    /// The amount of time after which the block will output false for the "is_valid" output if it has not successfully parsed from the input
    stale_age: Duration,
}

impl Parameters {
    pub fn new<S: AsRef<str>>(delimiter: &str, desired_outputs: &[S], stale_age_ms: f64) -> Self {
        // TODO: For now this accepts the normal desired output spec even though it only uses the indexes from it
        // The actual Datatypes are encoded as part of the type of the block, would make sense to address this more completely when
        // we rework codegen
        let desired_output_idx = Self::parse_desired_outputs(desired_outputs);
        Self {
            delimiter: delimiter.to_owned(),
            desired_output_idx,
            stale_age: Duration::from_secs_f64(stale_age_ms / 1000.0),
        }
    }

    fn parse_desired_outputs<S: AsRef<str>>(desired_outputs: &[S]) -> Vec<usize> {
        desired_outputs
            .iter()
            .map(|output_spec| {
                let (_data_type, idx) = output_spec
                    .as_ref()
                    .split_once(':')
                    .expect("Invalid output spec");
                idx.parse().expect("Invalid index, must be a number")
            })
            .collect()
    }
}

impl<T: Apply> Default for BytesSplitBlock<T> {
    fn default() -> Self {
        Self {
            data: T::default_block_data(),
            buffer: None,
            last_valid_time: None,
        }
    }
}

impl<T: Apply> ProcessBlock for BytesSplitBlock<T> {
    type Inputs = ByteSliceSignal;
    type Output = T::Output;
    type Parameters = Parameters;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        let parsed_deliminator = parse_string_to_read_delimiter(&parameters.delimiter);
        let parsed_deliminator = {
            let delim_len = parsed_deliminator.0.len() + parsed_deliminator.1.len();
            (parsed_deliminator.0, parsed_deliminator.1, delim_len)
        };
        let delim_idxs = find_all_bytes_idx(inputs, &parsed_deliminator.0, &parsed_deliminator.1);

        let update_age = context.time() - self.last_valid_time.unwrap_or_default();

        let parse_success = T::apply(
            &mut self.buffer,
            inputs,
            parameters,
            &delim_idxs,
            parsed_deliminator.2,
            update_age,
        );
        if parse_success {
            self.last_valid_time = Some(context.time());
        }
        self.data = T::build_block_data(self.buffer.as_ref().unwrap());
        <T as Apply>::storage_as_by(self.buffer.as_ref().unwrap())
    }
}

impl<T: Apply> IsValid for BytesSplitBlock<T> {
    fn is_valid(&self, _app_time_s: f64) -> OldBlockData {
        <OldBlockData as FromPass<bool>>::from_pass(T::is_valid(&self.buffer))
    }
}

pub trait FromBytes: DefaultStorage {
    fn from_bytes(bytes: &[u8]) -> Option<Self::Storage>;
}

impl FromBytes for ByteSliceSignal {
    fn from_bytes(bytes: &[u8]) -> Option<Self::Storage> {
        Some(bytes.to_owned())
    }
}

impl<T: Scalar + core::str::FromStr> FromBytes for T {
    fn from_bytes(bytes: &[u8]) -> Option<Self::Storage> {
        let scalar_str = core::str::from_utf8(bytes).ok()?;
        scalar_str.parse().ok()
    }
}

fn parse_bytes<T: FromBytes>(
    bytes: &[u8],
    delim_idxs: &[usize],
    desired_idx: usize,
    delim_len: usize,
) -> Option<T::Storage> {
    let start_byte = if desired_idx == 0 {
        0
    } else {
        *delim_idxs.get(desired_idx - 1)? + delim_len
    };
    let end_byte = if desired_idx < delim_idxs.len() {
        *delim_idxs.get(desired_idx)?
    } else {
        bytes.len()
    };
    T::from_bytes(&bytes[start_byte..end_byte])
}

pub trait Apply: Pass {
    /// The type of the storage for the block. This will be the storage type for each of the outputs plus a bool all in a tuple
    /// The bool is used for the always include `is_valid` output
    type Storage;

    type Output: Pass;

    /// Handles parsing input into the storage type, also manages the is_valid output. Outputs `true` if it was able to parse the input
    /// and `false` if it was not able to parse the input. This is used to update the `last_valid_time` field of the block.
    fn apply(
        dest: &mut Option<Self::Storage>,
        input_bytes: &[u8],
        params: &Parameters,
        delim_idxs: &[usize],
        delim_len: usize,
        update_age: Duration,
    ) -> bool;

    fn storage_as_by(storage: &Self::Storage) -> PassBy<'_, Self::Output>;

    fn build_block_data(storage: &Self::Storage) -> Vec<OldBlockData>;

    fn default_block_data() -> Vec<OldBlockData>;

    fn is_valid(storage: &Option<Self::Storage>) -> bool;
}

impl<A: FromBytes> Apply for A
where
    OldBlockData: FromPass<A>,
{
    type Storage = (A::Storage, bool);
    type Output = (A, bool);

    fn apply(
        dest: &mut Option<Self::Storage>,
        input_bytes: &[u8],
        params: &Parameters,
        delim_idxs: &[usize],
        delim_len: usize,
        update_age: Duration,
    ) -> bool {
        let dest = dest.get_or_insert((A::default_storage(), false));
        let parsed_data = parse_bytes::<A>(
            input_bytes,
            delim_idxs,
            params.desired_output_idx[0],
            delim_len,
        );
        if let Some(parsed_data) = parsed_data {
            *dest = (parsed_data, true);
            true
        } else {
            if update_age > params.stale_age {
                dest.1 = false;
            }
            false
        }
    }

    fn build_block_data(storage: &Self::Storage) -> Vec<OldBlockData> {
        vec![OldBlockData::from_pass(A::from_storage(&storage.0))]
    }

    fn default_block_data() -> Vec<OldBlockData> {
        vec![OldBlockData::from_pass(A::from_storage(
            &A::default_storage(),
        ))]
    }

    fn storage_as_by(storage: &Self::Storage) -> PassBy<'_, Self::Output> {
        (A::from_storage(&storage.0), storage.1)
    }

    fn is_valid(storage: &Option<Self::Storage>) -> bool {
        storage.as_ref().map(|s| s.1).unwrap_or(false)
    }
}

impl<A: FromBytes, B: FromBytes> Apply for (A, B)
where
    OldBlockData: FromPass<A>,
    OldBlockData: FromPass<B>,
{
    type Output = (A, B, bool);
    type Storage = (A::Storage, B::Storage, bool);

    fn apply(
        dest: &mut Option<Self::Storage>,
        input_bytes: &[u8],
        params: &Parameters,
        delim_idxs: &[usize],
        delim_len: usize,
        update_age: Duration,
    ) -> bool {
        let dest = dest.get_or_insert((A::default_storage(), B::default_storage(), false));
        let parsed_data_a = parse_bytes::<A>(
            input_bytes,
            delim_idxs,
            params.desired_output_idx[0],
            delim_len,
        );
        let parsed_data_b = parse_bytes::<B>(
            input_bytes,
            delim_idxs,
            params.desired_output_idx[1],
            delim_len,
        );
        if let (Some(parsed_data_a), Some(parsed_data_b)) = (parsed_data_a, parsed_data_b) {
            *dest = (parsed_data_a, parsed_data_b, true);
            true
        } else {
            if update_age > params.stale_age {
                dest.2 = false;
            }
            false
        }
    }

    fn build_block_data(storage: &Self::Storage) -> Vec<OldBlockData> {
        vec![
            <OldBlockData as FromPass<A>>::from_pass(A::from_storage(&storage.0)),
            <OldBlockData as FromPass<B>>::from_pass(B::from_storage(&storage.1)),
        ]
    }

    fn default_block_data() -> Vec<OldBlockData> {
        vec![
            <OldBlockData as FromPass<A>>::from_pass(A::from_storage(&A::default_storage())),
            <OldBlockData as FromPass<B>>::from_pass(B::from_storage(&B::default_storage())),
        ]
    }

    fn storage_as_by(storage: &Self::Storage) -> PassBy<'_, Self::Output> {
        (
            A::from_storage(&storage.0),
            B::from_storage(&storage.1),
            storage.2,
        )
    }
    fn is_valid(storage: &Option<Self::Storage>) -> bool {
        storage.as_ref().map(|s| s.2).unwrap_or(false)
    }
}

impl<A: FromBytes, B: FromBytes, C: FromBytes> Apply for (A, B, C)
where
    OldBlockData: FromPass<A>,
    OldBlockData: FromPass<B>,
    OldBlockData: FromPass<C>,
{
    type Output = (A, B, C, bool);
    type Storage = (A::Storage, B::Storage, C::Storage, bool);

    fn apply(
        dest: &mut Option<Self::Storage>,
        input_bytes: &[u8],
        params: &Parameters,
        delim_idxs: &[usize],
        delim_len: usize,
        update_age: Duration,
    ) -> bool {
        let dest = dest.get_or_insert((
            A::default_storage(),
            B::default_storage(),
            C::default_storage(),
            false,
        ));
        let parsed_data_a = parse_bytes::<A>(
            input_bytes,
            delim_idxs,
            params.desired_output_idx[0],
            delim_len,
        );
        let parsed_data_b = parse_bytes::<B>(
            input_bytes,
            delim_idxs,
            params.desired_output_idx[1],
            delim_len,
        );
        let parsed_data_c = parse_bytes::<C>(
            input_bytes,
            delim_idxs,
            params.desired_output_idx[2],
            delim_len,
        );
        if let (Some(parsed_data_a), Some(parsed_data_b), Some(parsed_data_c)) =
            (parsed_data_a, parsed_data_b, parsed_data_c)
        {
            *dest = (parsed_data_a, parsed_data_b, parsed_data_c, true);
            true
        } else {
            if update_age > params.stale_age {
                dest.3 = false;
            }
            false
        }
    }

    fn build_block_data(storage: &Self::Storage) -> Vec<OldBlockData> {
        vec![
            <OldBlockData as FromPass<A>>::from_pass(A::from_storage(&storage.0)),
            <OldBlockData as FromPass<B>>::from_pass(B::from_storage(&storage.1)),
            <OldBlockData as FromPass<C>>::from_pass(C::from_storage(&storage.2)),
        ]
    }

    fn default_block_data() -> Vec<OldBlockData> {
        vec![
            <OldBlockData as FromPass<A>>::from_pass(A::from_storage(&A::default_storage())),
            <OldBlockData as FromPass<B>>::from_pass(B::from_storage(&B::default_storage())),
            <OldBlockData as FromPass<C>>::from_pass(C::from_storage(&C::default_storage())),
        ]
    }

    fn storage_as_by(storage: &Self::Storage) -> PassBy<'_, Self::Output> {
        (
            A::from_storage(&storage.0),
            B::from_storage(&storage.1),
            C::from_storage(&storage.2),
            storage.3,
        )
    }
    fn is_valid(storage: &Option<Self::Storage>) -> bool {
        storage.as_ref().map(|s| s.3).unwrap_or(false)
    }
}

impl<A: FromBytes, B: FromBytes, C: FromBytes, D: FromBytes> Apply for (A, B, C, D)
where
    OldBlockData: FromPass<A>,
    OldBlockData: FromPass<B>,
    OldBlockData: FromPass<C>,
    OldBlockData: FromPass<D>,
{
    type Output = (A, B, C, D, bool);
    type Storage = (A::Storage, B::Storage, C::Storage, D::Storage, bool);

    fn apply(
        dest: &mut Option<Self::Storage>,
        input_bytes: &[u8],
        params: &Parameters,
        delim_idxs: &[usize],
        delim_len: usize,
        update_age: Duration,
    ) -> bool {
        let dest = dest.get_or_insert((
            A::default_storage(),
            B::default_storage(),
            C::default_storage(),
            D::default_storage(),
            false,
        ));
        let parsed_data_a = parse_bytes::<A>(
            input_bytes,
            delim_idxs,
            params.desired_output_idx[0],
            delim_len,
        );
        let parsed_data_b = parse_bytes::<B>(
            input_bytes,
            delim_idxs,
            params.desired_output_idx[1],
            delim_len,
        );
        let parsed_data_c = parse_bytes::<C>(
            input_bytes,
            delim_idxs,
            params.desired_output_idx[2],
            delim_len,
        );
        let parsed_data_d = parse_bytes::<D>(
            input_bytes,
            delim_idxs,
            params.desired_output_idx[3],
            delim_len,
        );
        if let (
            Some(parsed_data_a),
            Some(parsed_data_b),
            Some(parsed_data_c),
            Some(parsed_data_d),
        ) = (parsed_data_a, parsed_data_b, parsed_data_c, parsed_data_d)
        {
            *dest = (
                parsed_data_a,
                parsed_data_b,
                parsed_data_c,
                parsed_data_d,
                true,
            );
            true
        } else {
            if update_age > params.stale_age {
                dest.4 = false;
            }
            false
        }
    }

    fn build_block_data(storage: &Self::Storage) -> Vec<OldBlockData> {
        vec![
            <OldBlockData as FromPass<A>>::from_pass(A::from_storage(&storage.0)),
            <OldBlockData as FromPass<B>>::from_pass(B::from_storage(&storage.1)),
            <OldBlockData as FromPass<C>>::from_pass(C::from_storage(&storage.2)),
            <OldBlockData as FromPass<D>>::from_pass(D::from_storage(&storage.3)),
        ]
    }

    fn default_block_data() -> Vec<OldBlockData> {
        vec![
            <OldBlockData as FromPass<A>>::from_pass(A::from_storage(&A::default_storage())),
            <OldBlockData as FromPass<B>>::from_pass(B::from_storage(&B::default_storage())),
            <OldBlockData as FromPass<C>>::from_pass(C::from_storage(&C::default_storage())),
            <OldBlockData as FromPass<D>>::from_pass(D::from_storage(&D::default_storage())),
        ]
    }

    fn storage_as_by(storage: &Self::Storage) -> PassBy<'_, Self::Output> {
        (
            A::from_storage(&storage.0),
            B::from_storage(&storage.1),
            C::from_storage(&storage.2),
            D::from_storage(&storage.3),
            storage.4,
        )
    }

    fn is_valid(storage: &Option<Self::Storage>) -> bool {
        storage.as_ref().map(|s| s.4).unwrap_or(false)
    }
}

impl<A: FromBytes, B: FromBytes, C: FromBytes, D: FromBytes, E: FromBytes> Apply for (A, B, C, D, E)
where
    OldBlockData: FromPass<A>,
    OldBlockData: FromPass<B>,
    OldBlockData: FromPass<C>,
    OldBlockData: FromPass<D>,
    OldBlockData: FromPass<E>,
{
    type Output = (A, B, C, D, E, bool);
    type Storage = (
        A::Storage,
        B::Storage,
        C::Storage,
        D::Storage,
        E::Storage,
        bool,
    );

    fn apply(
        dest: &mut Option<Self::Storage>,
        input_bytes: &[u8],
        params: &Parameters,
        delim_idxs: &[usize],
        delim_len: usize,
        update_age: Duration,
    ) -> bool {
        let dest = dest.get_or_insert((
            A::default_storage(),
            B::default_storage(),
            C::default_storage(),
            D::default_storage(),
            E::default_storage(),
            false,
        ));
        let parsed_data_a = parse_bytes::<A>(
            input_bytes,
            delim_idxs,
            params.desired_output_idx[0],
            delim_len,
        );
        let parsed_data_b = parse_bytes::<B>(
            input_bytes,
            delim_idxs,
            params.desired_output_idx[1],
            delim_len,
        );
        let parsed_data_c = parse_bytes::<C>(
            input_bytes,
            delim_idxs,
            params.desired_output_idx[2],
            delim_len,
        );
        let parsed_data_d = parse_bytes::<D>(
            input_bytes,
            delim_idxs,
            params.desired_output_idx[3],
            delim_len,
        );
        let parsed_data_e = parse_bytes::<E>(
            input_bytes,
            delim_idxs,
            params.desired_output_idx[4],
            delim_len,
        );
        if let (
            Some(parsed_data_a),
            Some(parsed_data_b),
            Some(parsed_data_c),
            Some(parsed_data_d),
            Some(parsed_data_e),
        ) = (
            parsed_data_a,
            parsed_data_b,
            parsed_data_c,
            parsed_data_d,
            parsed_data_e,
        ) {
            *dest = (
                parsed_data_a,
                parsed_data_b,
                parsed_data_c,
                parsed_data_d,
                parsed_data_e,
                true,
            );
            true
        } else {
            if update_age > params.stale_age {
                dest.5 = false;
            }
            false
        }
    }

    fn build_block_data(storage: &Self::Storage) -> Vec<OldBlockData> {
        vec![
            <OldBlockData as FromPass<A>>::from_pass(A::from_storage(&storage.0)),
            <OldBlockData as FromPass<B>>::from_pass(B::from_storage(&storage.1)),
            <OldBlockData as FromPass<C>>::from_pass(C::from_storage(&storage.2)),
            <OldBlockData as FromPass<D>>::from_pass(D::from_storage(&storage.3)),
            <OldBlockData as FromPass<E>>::from_pass(E::from_storage(&storage.4)),
        ]
    }

    fn default_block_data() -> Vec<OldBlockData> {
        vec![
            <OldBlockData as FromPass<A>>::from_pass(A::from_storage(&A::default_storage())),
            <OldBlockData as FromPass<B>>::from_pass(B::from_storage(&B::default_storage())),
            <OldBlockData as FromPass<C>>::from_pass(C::from_storage(&C::default_storage())),
            <OldBlockData as FromPass<D>>::from_pass(D::from_storage(&D::default_storage())),
            <OldBlockData as FromPass<E>>::from_pass(E::from_storage(&E::default_storage())),
        ]
    }

    fn storage_as_by(storage: &Self::Storage) -> PassBy<'_, Self::Output> {
        (
            A::from_storage(&storage.0),
            B::from_storage(&storage.1),
            C::from_storage(&storage.2),
            D::from_storage(&storage.3),
            E::from_storage(&storage.4),
            storage.5,
        )
    }

    fn is_valid(storage: &Option<Self::Storage>) -> bool {
        storage.as_ref().map(|s| s.5).unwrap_or(false)
    }
}

impl<A: FromBytes, B: FromBytes, C: FromBytes, D: FromBytes, E: FromBytes, F: FromBytes> Apply
    for (A, B, C, D, E, F)
where
    OldBlockData: FromPass<A>,
    OldBlockData: FromPass<B>,
    OldBlockData: FromPass<C>,
    OldBlockData: FromPass<D>,
    OldBlockData: FromPass<E>,
    OldBlockData: FromPass<F>,
{
    type Output = (A, B, C, D, E, F, bool);
    type Storage = (
        A::Storage,
        B::Storage,
        C::Storage,
        D::Storage,
        E::Storage,
        F::Storage,
        bool,
    );

    fn apply(
        dest: &mut Option<Self::Storage>,
        input_bytes: &[u8],
        params: &Parameters,
        delim_idxs: &[usize],
        delim_len: usize,
        update_age: Duration,
    ) -> bool {
        let dest = dest.get_or_insert((
            A::default_storage(),
            B::default_storage(),
            C::default_storage(),
            D::default_storage(),
            E::default_storage(),
            F::default_storage(),
            false,
        ));
        let parsed_data_a = parse_bytes::<A>(
            input_bytes,
            delim_idxs,
            params.desired_output_idx[0],
            delim_len,
        );
        let parsed_data_b = parse_bytes::<B>(
            input_bytes,
            delim_idxs,
            params.desired_output_idx[1],
            delim_len,
        );
        let parsed_data_c = parse_bytes::<C>(
            input_bytes,
            delim_idxs,
            params.desired_output_idx[2],
            delim_len,
        );
        let parsed_data_d = parse_bytes::<D>(
            input_bytes,
            delim_idxs,
            params.desired_output_idx[3],
            delim_len,
        );
        let parsed_data_e = parse_bytes::<E>(
            input_bytes,
            delim_idxs,
            params.desired_output_idx[4],
            delim_len,
        );
        let parsed_data_f = parse_bytes::<F>(
            input_bytes,
            delim_idxs,
            params.desired_output_idx[5],
            delim_len,
        );
        if let (
            Some(parsed_data_a),
            Some(parsed_data_b),
            Some(parsed_data_c),
            Some(parsed_data_d),
            Some(parsed_data_e),
            Some(parsed_data_f),
        ) = (
            parsed_data_a,
            parsed_data_b,
            parsed_data_c,
            parsed_data_d,
            parsed_data_e,
            parsed_data_f,
        ) {
            *dest = (
                parsed_data_a,
                parsed_data_b,
                parsed_data_c,
                parsed_data_d,
                parsed_data_e,
                parsed_data_f,
                true,
            );
            true
        } else {
            if update_age > params.stale_age {
                dest.6 = false;
            }
            false
        }
    }

    fn build_block_data(storage: &Self::Storage) -> Vec<OldBlockData> {
        vec![
            <OldBlockData as FromPass<A>>::from_pass(A::from_storage(&storage.0)),
            <OldBlockData as FromPass<B>>::from_pass(B::from_storage(&storage.1)),
            <OldBlockData as FromPass<C>>::from_pass(C::from_storage(&storage.2)),
            <OldBlockData as FromPass<D>>::from_pass(D::from_storage(&storage.3)),
            <OldBlockData as FromPass<E>>::from_pass(E::from_storage(&storage.4)),
            <OldBlockData as FromPass<F>>::from_pass(F::from_storage(&storage.5)),
        ]
    }

    fn default_block_data() -> Vec<OldBlockData> {
        vec![
            <OldBlockData as FromPass<A>>::from_pass(A::from_storage(&A::default_storage())),
            <OldBlockData as FromPass<B>>::from_pass(B::from_storage(&B::default_storage())),
            <OldBlockData as FromPass<C>>::from_pass(C::from_storage(&C::default_storage())),
            <OldBlockData as FromPass<D>>::from_pass(D::from_storage(&D::default_storage())),
            <OldBlockData as FromPass<E>>::from_pass(E::from_storage(&E::default_storage())),
            <OldBlockData as FromPass<F>>::from_pass(F::from_storage(&F::default_storage())),
        ]
    }

    fn storage_as_by(storage: &Self::Storage) -> PassBy<'_, Self::Output> {
        (
            A::from_storage(&storage.0),
            B::from_storage(&storage.1),
            C::from_storage(&storage.2),
            D::from_storage(&storage.3),
            E::from_storage(&storage.4),
            F::from_storage(&storage.5),
            storage.6,
        )
    }

    fn is_valid(storage: &Option<Self::Storage>) -> bool {
        storage.as_ref().map(|s| s.6).unwrap_or(false)
    }
}

impl<
        A: FromBytes,
        B: FromBytes,
        C: FromBytes,
        D: FromBytes,
        E: FromBytes,
        F: FromBytes,
        G: FromBytes,
    > Apply for (A, B, C, D, E, F, G)
where
    OldBlockData: FromPass<A>,
    OldBlockData: FromPass<B>,
    OldBlockData: FromPass<C>,
    OldBlockData: FromPass<D>,
    OldBlockData: FromPass<E>,
    OldBlockData: FromPass<F>,
    OldBlockData: FromPass<G>,
{
    type Output = (A, B, C, D, E, F, G, bool);
    type Storage = (
        A::Storage,
        B::Storage,
        C::Storage,
        D::Storage,
        E::Storage,
        F::Storage,
        G::Storage,
        bool,
    );

    fn apply(
        dest: &mut Option<Self::Storage>,
        input_bytes: &[u8],
        params: &Parameters,
        delim_idxs: &[usize],
        delim_len: usize,
        update_age: Duration,
    ) -> bool {
        let dest = dest.get_or_insert((
            A::default_storage(),
            B::default_storage(),
            C::default_storage(),
            D::default_storage(),
            E::default_storage(),
            F::default_storage(),
            G::default_storage(),
            false,
        ));
        let parsed_data_a = parse_bytes::<A>(
            input_bytes,
            delim_idxs,
            params.desired_output_idx[0],
            delim_len,
        );
        let parsed_data_b = parse_bytes::<B>(
            input_bytes,
            delim_idxs,
            params.desired_output_idx[1],
            delim_len,
        );
        let parsed_data_c = parse_bytes::<C>(
            input_bytes,
            delim_idxs,
            params.desired_output_idx[2],
            delim_len,
        );
        let parsed_data_d = parse_bytes::<D>(
            input_bytes,
            delim_idxs,
            params.desired_output_idx[3],
            delim_len,
        );
        let parsed_data_e = parse_bytes::<E>(
            input_bytes,
            delim_idxs,
            params.desired_output_idx[4],
            delim_len,
        );
        let parsed_data_f = parse_bytes::<F>(
            input_bytes,
            delim_idxs,
            params.desired_output_idx[5],
            delim_len,
        );
        let parsed_data_g = parse_bytes::<G>(
            input_bytes,
            delim_idxs,
            params.desired_output_idx[6],
            delim_len,
        );
        if let (
            Some(parsed_data_a),
            Some(parsed_data_b),
            Some(parsed_data_c),
            Some(parsed_data_d),
            Some(parsed_data_e),
            Some(parsed_data_f),
            Some(parsed_data_g),
        ) = (
            parsed_data_a,
            parsed_data_b,
            parsed_data_c,
            parsed_data_d,
            parsed_data_e,
            parsed_data_f,
            parsed_data_g,
        ) {
            *dest = (
                parsed_data_a,
                parsed_data_b,
                parsed_data_c,
                parsed_data_d,
                parsed_data_e,
                parsed_data_f,
                parsed_data_g,
                true,
            );
            true
        } else {
            if update_age > params.stale_age {
                dest.7 = false;
            }
            false
        }
    }

    fn build_block_data(storage: &Self::Storage) -> Vec<OldBlockData> {
        vec![
            <OldBlockData as FromPass<A>>::from_pass(A::from_storage(&storage.0)),
            <OldBlockData as FromPass<B>>::from_pass(B::from_storage(&storage.1)),
            <OldBlockData as FromPass<C>>::from_pass(C::from_storage(&storage.2)),
            <OldBlockData as FromPass<D>>::from_pass(D::from_storage(&storage.3)),
            <OldBlockData as FromPass<E>>::from_pass(E::from_storage(&storage.4)),
            <OldBlockData as FromPass<F>>::from_pass(F::from_storage(&storage.5)),
            <OldBlockData as FromPass<G>>::from_pass(G::from_storage(&storage.6)),
        ]
    }

    fn default_block_data() -> Vec<OldBlockData> {
        vec![
            <OldBlockData as FromPass<A>>::from_pass(A::from_storage(&A::default_storage())),
            <OldBlockData as FromPass<B>>::from_pass(B::from_storage(&B::default_storage())),
            <OldBlockData as FromPass<C>>::from_pass(C::from_storage(&C::default_storage())),
            <OldBlockData as FromPass<D>>::from_pass(D::from_storage(&D::default_storage())),
            <OldBlockData as FromPass<E>>::from_pass(E::from_storage(&E::default_storage())),
            <OldBlockData as FromPass<F>>::from_pass(F::from_storage(&F::default_storage())),
            <OldBlockData as FromPass<G>>::from_pass(G::from_storage(&G::default_storage())),
        ]
    }

    fn storage_as_by(storage: &Self::Storage) -> PassBy<'_, Self::Output> {
        (
            A::from_storage(&storage.0),
            B::from_storage(&storage.1),
            C::from_storage(&storage.2),
            D::from_storage(&storage.3),
            E::from_storage(&storage.4),
            F::from_storage(&storage.5),
            G::from_storage(&storage.6),
            storage.7,
        )
    }

    fn is_valid(storage: &Option<Self::Storage>) -> bool {
        storage.as_ref().map(|s| s.7).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;

    // TODO: We have tests for each impl of Apply, but don't yet test their handling of the stale_age parameter

    #[test]
    fn test_bytes_split_block_data() {
        let mut context = StubContext::default();
        let params = Parameters::new(":", &["scalar:0", "scalar:3", "BytesArray:1"], 1000.0);
        let mut block = BytesSplitBlock::<(f64, f64, ByteSliceSignal)>::default();
        let input = br#"123:4.56:78.9:42.0"#;
        let output = block.process(&params, &context, input);
        assert_eq!(output, (123.0, 42.0, b"4.56".as_slice(), true));
        assert_eq!(block.data[0].scalar(), 123.0);
        assert_eq!(block.data[1].scalar(), 42.0);
        assert_eq!(block.data[2].to_bytes(), b"4.56");
        assert_eq!(block.data.len(), 3);
        assert_ne!(block.is_valid(0.0).scalar(), 0.0);

        context.time += context.fundamental_timestep;
        let input = br#"123:4.56:78.9"#;
        let output = block.process(&params, &context, input);
        assert_eq!(output, (123.0, 42.0, b"4.56".as_slice(), true)); // stale time has not elapsed
        assert_eq!(block.data[0].scalar(), 123.0);
        assert_eq!(block.data[1].scalar(), 42.0);
        assert_eq!(block.data[2].to_bytes(), b"4.56");
        assert_eq!(block.data.len(), 3);
        assert_ne!(block.is_valid(0.0).scalar(), 0.0);

        context.time = Duration::from_secs_f64(2.0);
        let output = block.process(&params, &context, input);
        assert_eq!(output, (123.0, 42.0, b"4.56".as_slice(), false)); // stale time has elapsed
        assert_eq!(block.data[0].scalar(), 123.0);
        assert_eq!(block.data[1].scalar(), 42.0);
        assert_eq!(block.data[2].to_bytes(), b"4.56");
        assert_eq!(block.data.len(), 3);
        assert_eq!(block.is_valid(0.0).scalar(), 0.0);

        // Now input is valid again
        context.time += context.fundamental_timestep;
        let input = br#"1.03:17:23.4:11.0"#;
        let output = block.process(&params, &context, input);
        assert_eq!(output, (1.03, 11.0, b"17".as_slice(), true));
        assert_eq!(block.data[0].scalar(), 1.03);
        assert_eq!(block.data[1].scalar(), 11.0);
        assert_eq!(block.data[2].to_bytes(), b"17");
        assert_eq!(block.data.len(), 3);
        assert_ne!(block.is_valid(0.0).scalar(), 0.0);
    }

    #[test]
    fn test_wildcard_delim() {
        let context = StubContext::default();
        let delim = r"\xAA\x**\xAB";
        let parameters = Parameters::new(delim, &["BytesArray:0", "BytesArray:2"], 1000.0);
        let mut block = BytesSplitBlock::<(ByteSliceSignal, ByteSliceSignal)>::default();

        let input = b"\x00\xAA\xAA\xAB\x01\xAA\xFF\xAB\x02";
        let output = block.process(&parameters, &context, input);
        assert_eq!(output, (b"\x00".as_ref(), b"\x02".as_ref(), true));
        assert_eq!(block.data[0].to_bytes(), b"\x00");
        assert_eq!(block.data[1].to_bytes(), b"\x02");
        assert_eq!(block.data.len(), 2);
        assert_ne!(block.is_valid(0.0).scalar(), 0.0);
    }

    #[test]
    fn test_1_output() {
        let context = StubContext::default();
        let parameters = Parameters::new(":", &["scalar:3"], 1000.0);
        let mut block = BytesSplitBlock::<f64>::default();

        let input = b"123:4.56:78.9:42.0";
        let output = block.process(&parameters, &context, input);
        assert_eq!(output, (42.0, true));
        assert_eq!(block.data[0].scalar(), 42.0);
        assert_eq!(block.data.len(), 1);
        assert_ne!(block.is_valid(0.0).scalar(), 0.0);
    }

    #[test]
    fn test_2_outputs() {
        let context = StubContext::default();
        let parameters = Parameters::new(":", &["scalar:0", "BytesArray:3"], 1000.0);
        let mut block = BytesSplitBlock::<(f64, ByteSliceSignal)>::default();

        let input = b"123:4.56:78.9:42.0";
        let output = block.process(&parameters, &context, input);
        assert_eq!(output, (123.0, b"42.0".as_slice(), true));
        assert_eq!(block.data[0].scalar(), 123.0);
        assert_eq!(block.data[1].to_bytes(), b"42.0");
        assert_eq!(block.data.len(), 2);
        assert_ne!(block.is_valid(0.0).scalar(), 0.0);
    }

    #[test]
    fn test_3_outputs() {
        let context = StubContext::default();
        let parameters = Parameters::new(":", &["scalar:0", "scalar:3", "BytesArray:1"], 1000.0);
        let mut block = BytesSplitBlock::<(f64, f64, ByteSliceSignal)>::default();

        let input = b"123:4.56:78.9:42.0";
        let output = block.process(&parameters, &context, input);
        assert_eq!(output, (123.0, 42.0, b"4.56".as_slice(), true));
        assert_eq!(block.data[0].scalar(), 123.0);
        assert_eq!(block.data[1].scalar(), 42.0);
        assert_eq!(block.data[2].to_bytes(), b"4.56");
        assert_eq!(block.data.len(), 3);
        assert_ne!(block.is_valid(0.0).scalar(), 0.0);
    }

    #[test]
    fn test_4_outputs() {
        let context = StubContext::default();
        let parameters = Parameters::new(
            ":",
            &["scalar:0", "scalar:3", "scalar:1", "BytesArray:2"],
            1000.0,
        );
        let mut block = BytesSplitBlock::<(f64, f64, f64, ByteSliceSignal)>::default();

        let input = b"123:4.56:78.9:42.0";
        let output = block.process(&parameters, &context, input);
        assert_eq!(output, (123.0, 42.0, 4.56, b"78.9".as_slice(), true));
        assert_eq!(block.data[0].scalar(), 123.0);
        assert_eq!(block.data[1].scalar(), 42.0);
        assert_eq!(block.data[2].scalar(), 4.56);
        assert_eq!(block.data[3].to_bytes(), b"78.9");
        assert_eq!(block.data.len(), 4);
        assert_ne!(block.is_valid(0.0).scalar(), 0.0);
    }

    #[test]
    fn test_5_outputs() {
        let context = StubContext::default();
        let parameters = Parameters::new(
            ":",
            &[
                "scalar:0",
                "scalar:3",
                "scalar:1",
                "scalar:2",
                "BytesArray:2",
            ],
            1000.0,
        );
        let mut block = BytesSplitBlock::<(f64, f64, f64, f64, ByteSliceSignal)>::default();

        let input = b"123:4.56:78.9:42.0";
        let output = block.process(&parameters, &context, input);
        assert_eq!(output, (123.0, 42.0, 4.56, 78.9, b"78.9".as_slice(), true));
        assert_eq!(block.data[0].scalar(), 123.0);
        assert_eq!(block.data[1].scalar(), 42.0);
        assert_eq!(block.data[2].scalar(), 4.56);
        assert_eq!(block.data[3].scalar(), 78.9);
        assert_eq!(block.data[4].to_bytes(), b"78.9");
        assert_eq!(block.data.len(), 5);
        assert_ne!(block.is_valid(0.0).scalar(), 0.0);
    }

    #[test]
    fn test_6_outputs() {
        let context = StubContext::default();
        let parameters = Parameters::new(
            ":",
            &[
                "scalar:0",
                "scalar:3",
                "BytesArray:2",
                "scalar:1",
                "scalar:2",
                "scalar:1",
            ],
            1000.0,
        );
        let mut block = BytesSplitBlock::<(f64, f64, ByteSliceSignal, f64, f64, f64)>::default();

        let input = b"123:4.56:78.9:42.0";
        let output = block.process(&parameters, &context, input);
        assert_eq!(
            output,
            (123.0, 42.0, b"78.9".as_slice(), 4.56, 78.9, 4.56, true)
        );
        assert_eq!(block.data[0].scalar(), 123.0);
        assert_eq!(block.data[1].scalar(), 42.0);
        assert_eq!(block.data[2].to_bytes(), b"78.9");
        assert_eq!(block.data[3].scalar(), 4.56);
        assert_eq!(block.data[4].scalar(), 78.9);
        assert_eq!(block.data[5].scalar(), 4.56);

        assert_eq!(block.data.len(), 6);
        assert_ne!(block.is_valid(0.0).scalar(), 0.0);
    }

    #[test]
    fn test_7_outputs() {
        let context = StubContext::default();
        let parameters = Parameters::new(
            ":",
            &[
                "scalar:0",
                "scalar:3",
                "BytesArray:2",
                "scalar:1",
                "scalar:2",
                "scalar:1",
                "BytesArray:2",
            ],
            1000.0,
        );
        let mut block = BytesSplitBlock::<(
            f64,
            f64,
            ByteSliceSignal,
            f64,
            f64,
            f64,
            ByteSliceSignal,
        )>::default();

        let input = b"123:4.56:78.9:42.0";
        let output = block.process(&parameters, &context, input);
        assert_eq!(
            output,
            (
                123.0,
                42.0,
                b"78.9".as_slice(),
                4.56,
                78.9,
                4.56,
                b"78.9".as_slice(),
                true
            )
        );
        assert_eq!(block.data[0].scalar(), 123.0);
        assert_eq!(block.data[1].scalar(), 42.0);
        assert_eq!(block.data[2].to_bytes(), b"78.9");
        assert_eq!(block.data[3].scalar(), 4.56);
        assert_eq!(block.data[4].scalar(), 78.9);
        assert_eq!(block.data[5].scalar(), 4.56);
        assert_eq!(block.data[6].to_bytes(), b"78.9");

        assert_eq!(block.data.len(), 7);
        assert_ne!(block.is_valid(0.0).scalar(), 0.0);
    }
}
