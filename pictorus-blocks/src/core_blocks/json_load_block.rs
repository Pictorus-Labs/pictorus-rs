extern crate alloc;

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use miniserde::json::{self, Array, Number, Object, Value};
use pictorus_block_data::{BlockData as OldBlockData, BlockDataType, FromPass};
use pictorus_traits::{ByteSliceSignal, Context, Matrix, Pass, PassBy, ProcessBlock};

use crate::{stale_tracker::StaleTracker, traits::DefaultStorage, IsValid};

/// Deserializes bytes encoded as JSON into the specified output signals.
///
/// If select_data is provided in the parameters,
/// we assume that the passed in bytes represent an object where each key of the
/// select_data is a key in the object. If select_data is not provided, we assume
/// that the passed in bytes represent a single value (either scalar or matrix).
pub struct JsonLoadBlock<T: Apply> {
    pub data: Vec<OldBlockData>,
    buffer: T::Storage,
    stale_tracker: Option<StaleTracker>,
}

impl<T: Apply> Default for JsonLoadBlock<T> {
    fn default() -> Self {
        let buffer = T::default_storage();
        let data = T::build_block_data(&buffer);
        JsonLoadBlock {
            data,
            buffer,
            stale_tracker: None,
        }
    }
}

impl<T: Apply> ProcessBlock for JsonLoadBlock<T> {
    type Inputs = ByteSliceSignal;
    type Output = T::Output;
    type Parameters = Parameters;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        context: &dyn Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        let success = T::apply(&mut self.buffer, inputs, parameters);
        if success.is_ok() {
            self.data = T::build_block_data(&self.buffer);
            let tracker = self
                .stale_tracker
                .get_or_insert(StaleTracker::from_ms(parameters.stale_age_ms));
            tracker.mark_updated(context.time().as_secs_f64());
        }
        T::storage_as_by(&self.buffer)
    }
}

impl<T: Apply> IsValid for JsonLoadBlock<T> {
    fn is_valid(&self, app_time_s: f64) -> OldBlockData {
        match self.stale_tracker {
            Some(ref tracker) => tracker.is_valid(app_time_s),
            None => OldBlockData::scalar_from_bool(false),
        }
    }
}

/// Parameters for the JSON Load Block
pub struct Parameters {
    /// The select data is a list of tuples where the first element is the
    /// type of the output and the second element is the key in the JSON object
    /// Note: The data type is not actually used in code, but encoded into the
    /// generics of the block instance.
    pub select_data: Vec<(BlockDataType, String)>,
    /// The age in milliseconds after which the data is considered stale
    pub stale_age_ms: f64,
}

impl Parameters {
    // TODO: This should be changed to accept an &[&str]. In some other places we use a generic
    // `<S: AsRef<str>>` to allow for both &str and String. It's tricky to do that here because
    // we allow empty select_data, which would require us to specify a generic type.
    pub fn new(select_data: &[String], stale_age_ms: f64) -> Self {
        let select_data = Self::parse_select_spec(select_data);
        Self {
            select_data,
            stale_age_ms,
        }
    }

    fn parse_select_spec(data: &[String]) -> Vec<(BlockDataType, String)> {
        data.iter()
            .map(|d| d.split_once(':').expect("Invalid select data format"))
            .map(|(dt, field)| (dt.parse::<BlockDataType>().unwrap(), field.into()))
            .collect()
    }
}

pub trait Deserialize: DefaultStorage {
    fn from_json_value(data: &Value) -> Result<Self::Storage, ()>;
    fn from_json_object(data: &Object, key: &str) -> Result<Self::Storage, ()> {
        let value = data.get(key).ok_or(())?;
        Self::from_json_value(value)
    }
}

impl Deserialize for f64 {
    fn from_json_value(data: &Value) -> Result<Self::Storage, ()> {
        match data {
            Value::Number(n) => Ok(parse_number(n)),
            _ => Err(()),
        }
    }
}

impl Deserialize for ByteSliceSignal {
    fn from_json_value(data: &Value) -> Result<Self::Storage, ()> {
        match data {
            Value::String(s) => Ok(s.as_str().into()),
            _ => Err(()),
        }
    }
}

fn parse_num_array<const NROWS: usize, const NCOLS: usize>(
    data: &Array,
    res: &mut Matrix<NROWS, NCOLS, f64>,
) -> Result<(), ()> {
    // If ROWS * COLS is equal to the total number of elements in the matrix
    // then we can fill the matrix column-wise
    if data.len() != NROWS * NCOLS {
        return Err(());
    }

    data.iter().enumerate().for_each(|(i, v)| {
        if let Value::Number(n) = v {
            let col = i % NCOLS;
            let row = i / NCOLS;
            res.data[col][row] = parse_number(n);
        }
    });

    Ok(())
}

fn parse_nested_array<const NROWS: usize, const NCOLS: usize>(
    data: &Array,
    res: &mut Matrix<NROWS, NCOLS, f64>,
) -> Result<(), ()> {
    let rows = data.len();
    if rows != NROWS {
        return Err(());
    }

    for i in 0..NROWS {
        let row = match &data[i] {
            Value::Array(v) => v,
            _ => return Err(()),
        };
        if row.len() != NCOLS {
            return Err(());
        }
        for j in 0..NCOLS {
            match &row[j] {
                Value::Number(n) => {
                    res.data[j][i] = parse_number(n);
                }
                _ => return Err(()),
            }
        }
    }

    Ok(())
}

impl<const NROWS: usize, const NCOLS: usize> Deserialize for Matrix<NROWS, NCOLS, f64> {
    fn from_json_value(data: &Value) -> Result<Self::Storage, ()> {
        let mut res = Self::Storage::default();
        let val = match data {
            Value::Array(v) => v,
            _ => return Err(()),
        };

        if val.is_empty() {
            let res = if NROWS * NCOLS == 0 { Ok(res) } else { Err(()) };
            return res;
        }

        match &val[0] {
            Value::Number(_) => parse_num_array(val, &mut res),
            Value::Array(_) => parse_nested_array(val, &mut res),
            _ => Err(()),
        }?;
        Ok(res)
    }
}

pub trait Apply: Pass {
    type Storage;
    type Output: Pass;
    fn apply(
        dest: &mut Self::Storage,
        data: PassBy<ByteSliceSignal>,
        parameters: &Parameters,
    ) -> Result<(), ()>;

    fn default_storage() -> Self::Storage;
    fn storage_as_by(storage: &Self::Storage) -> PassBy<'_, Self::Output>;
    fn build_block_data(storage: &Self::Storage) -> Vec<OldBlockData>;
}

fn parse_number(num_val: &Number) -> f64 {
    match num_val {
        Number::F64(v) => *v,
        Number::I64(v) => *v as f64,
        Number::U64(v) => *v as f64,
    }
}

// Impl for single value
impl<A: Deserialize> Apply for A
where
    OldBlockData: FromPass<A>,
{
    type Storage = (A::Storage, bool);
    type Output = (A, bool);

    fn apply(
        dest: &mut Self::Storage,
        data: PassBy<ByteSliceSignal>,
        parameters: &Parameters,
    ) -> Result<(), ()> {
        let data = core::str::from_utf8(data).or(Err(()))?;
        // Special case for a single value where no selectors are provided
        // In this case we will attempt to parse the entire data as a single value
        let v1 = if parameters.select_data.is_empty() {
            let data: Value = json::from_str(data).or(Err(()))?;
            A::from_json_value(&data)
        } else {
            let data: Object = json::from_str(data).or(Err(()))?;
            A::from_json_object(&data, &parameters.select_data[0].1)
        };

        if let Ok(v1) = v1 {
            *dest = (v1, true);
            Ok(())
        } else {
            dest.1 = false;
            Err(())
        }
    }

    fn default_storage() -> Self::Storage {
        (A::default_storage(), false)
    }

    fn build_block_data(storage: &Self::Storage) -> Vec<OldBlockData> {
        vec![OldBlockData::from_pass(A::from_storage(&storage.0))]
    }

    fn storage_as_by(storage: &Self::Storage) -> PassBy<'_, Self::Output> {
        (A::from_storage(&storage.0), storage.1)
    }
}

// Impl for tuple of two values
impl<A: Deserialize, B: Deserialize> Apply for (A, B)
where
    OldBlockData: FromPass<A>,
    OldBlockData: FromPass<B>,
{
    type Storage = (A::Storage, B::Storage, bool);
    type Output = (A, B, bool);

    fn apply(
        dest: &mut Self::Storage,
        data: PassBy<ByteSliceSignal>,
        parameters: &Parameters,
    ) -> Result<(), ()> {
        let data = core::str::from_utf8(data).or(Err(()))?;
        let data: Object = json::from_str(data).or(Err(()))?;
        let v1 = A::from_json_object(&data, &parameters.select_data[0].1);
        let v2 = B::from_json_object(&data, &parameters.select_data[1].1);
        if let (Ok(v1), Ok(v2)) = (v1, v2) {
            *dest = (v1, v2, true);
            Ok(())
        } else {
            dest.2 = false;
            Err(())
        }
    }

    fn default_storage() -> Self::Storage {
        (A::default_storage(), B::default_storage(), false)
    }

    fn build_block_data(storage: &Self::Storage) -> Vec<OldBlockData> {
        vec![
            <OldBlockData as FromPass<A>>::from_pass(A::from_storage(&storage.0)),
            <OldBlockData as FromPass<B>>::from_pass(B::from_storage(&storage.1)),
        ]
    }

    fn storage_as_by(storage: &Self::Storage) -> PassBy<'_, Self::Output> {
        (
            A::from_storage(&storage.0),
            B::from_storage(&storage.1),
            storage.2,
        )
    }
}

// Impl for tuple of three values
impl<A: Deserialize, B: Deserialize, C: Deserialize> Apply for (A, B, C)
where
    OldBlockData: FromPass<A>,
    OldBlockData: FromPass<B>,
    OldBlockData: FromPass<C>,
{
    type Storage = (A::Storage, B::Storage, C::Storage, bool);
    type Output = (A, B, C, bool);

    fn apply(
        dest: &mut Self::Storage,
        data: PassBy<ByteSliceSignal>,
        parameters: &Parameters,
    ) -> Result<(), ()> {
        let data = core::str::from_utf8(data).or(Err(()))?;
        let data: Object = json::from_str(data).or(Err(()))?;
        let v1 = A::from_json_object(&data, &parameters.select_data[0].1);
        let v2 = B::from_json_object(&data, &parameters.select_data[1].1);
        let v3 = C::from_json_object(&data, &parameters.select_data[2].1);
        if let (Ok(v1), Ok(v2), Ok(v3)) = (v1, v2, v3) {
            *dest = (v1, v2, v3, true);
            Ok(())
        } else {
            dest.3 = false;
            Err(())
        }
    }

    fn default_storage() -> Self::Storage {
        (
            A::default_storage(),
            B::default_storage(),
            C::default_storage(),
            false,
        )
    }

    fn build_block_data(storage: &Self::Storage) -> Vec<OldBlockData> {
        vec![
            <OldBlockData as FromPass<A>>::from_pass(A::from_storage(&storage.0)),
            <OldBlockData as FromPass<B>>::from_pass(B::from_storage(&storage.1)),
            <OldBlockData as FromPass<C>>::from_pass(C::from_storage(&storage.2)),
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
}

// Impl for tuple of four values
impl<A: Deserialize, B: Deserialize, C: Deserialize, D: Deserialize> Apply for (A, B, C, D)
where
    OldBlockData: FromPass<A>,
    OldBlockData: FromPass<B>,
    OldBlockData: FromPass<C>,
    OldBlockData: FromPass<D>,
{
    type Storage = (A::Storage, B::Storage, C::Storage, D::Storage, bool);
    type Output = (A, B, C, D, bool);

    fn apply(
        dest: &mut Self::Storage,
        data: PassBy<ByteSliceSignal>,
        parameters: &Parameters,
    ) -> Result<(), ()> {
        let data = core::str::from_utf8(data).or(Err(()))?;
        let data: Object = json::from_str(data).or(Err(()))?;
        let v1 = A::from_json_object(&data, &parameters.select_data[0].1);
        let v2 = B::from_json_object(&data, &parameters.select_data[1].1);
        let v3 = C::from_json_object(&data, &parameters.select_data[2].1);
        let v4 = D::from_json_object(&data, &parameters.select_data[3].1);
        if let (Ok(v1), Ok(v2), Ok(v3), Ok(v4)) = (v1, v2, v3, v4) {
            *dest = (v1, v2, v3, v4, true);
            Ok(())
        } else {
            dest.4 = false;
            Err(())
        }
    }

    fn default_storage() -> Self::Storage {
        (
            A::default_storage(),
            B::default_storage(),
            C::default_storage(),
            D::default_storage(),
            false,
        )
    }

    fn build_block_data(storage: &Self::Storage) -> Vec<OldBlockData> {
        vec![
            <OldBlockData as FromPass<A>>::from_pass(A::from_storage(&storage.0)),
            <OldBlockData as FromPass<B>>::from_pass(B::from_storage(&storage.1)),
            <OldBlockData as FromPass<C>>::from_pass(C::from_storage(&storage.2)),
            <OldBlockData as FromPass<D>>::from_pass(D::from_storage(&storage.3)),
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
}

// Impl for tuple of five values
impl<A: Deserialize, B: Deserialize, C: Deserialize, D: Deserialize, E: Deserialize> Apply
    for (A, B, C, D, E)
where
    OldBlockData: FromPass<A>,
    OldBlockData: FromPass<B>,
    OldBlockData: FromPass<C>,
    OldBlockData: FromPass<D>,
    OldBlockData: FromPass<E>,
{
    type Storage = (
        A::Storage,
        B::Storage,
        C::Storage,
        D::Storage,
        E::Storage,
        bool,
    );
    type Output = (A, B, C, D, E, bool);

    fn apply(
        dest: &mut Self::Storage,
        data: PassBy<ByteSliceSignal>,
        parameters: &Parameters,
    ) -> Result<(), ()> {
        let data = core::str::from_utf8(data).or(Err(()))?;
        let data: Object = json::from_str(data).or(Err(()))?;
        let v1 = A::from_json_object(&data, &parameters.select_data[0].1);
        let v2 = B::from_json_object(&data, &parameters.select_data[1].1);
        let v3 = C::from_json_object(&data, &parameters.select_data[2].1);
        let v4 = D::from_json_object(&data, &parameters.select_data[3].1);
        let v5 = E::from_json_object(&data, &parameters.select_data[4].1);
        if let (Ok(v1), Ok(v2), Ok(v3), Ok(v4), Ok(v5)) = (v1, v2, v3, v4, v5) {
            *dest = (v1, v2, v3, v4, v5, true);
            Ok(())
        } else {
            dest.5 = false;
            Err(())
        }
    }

    fn default_storage() -> Self::Storage {
        (
            A::default_storage(),
            B::default_storage(),
            C::default_storage(),
            D::default_storage(),
            E::default_storage(),
            false,
        )
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
}

// Impl for tuple of six values
impl<
        A: Deserialize,
        B: Deserialize,
        C: Deserialize,
        D: Deserialize,
        E: Deserialize,
        F: Deserialize,
    > Apply for (A, B, C, D, E, F)
where
    OldBlockData: FromPass<A>,
    OldBlockData: FromPass<B>,
    OldBlockData: FromPass<C>,
    OldBlockData: FromPass<D>,
    OldBlockData: FromPass<E>,
    OldBlockData: FromPass<F>,
{
    type Storage = (
        A::Storage,
        B::Storage,
        C::Storage,
        D::Storage,
        E::Storage,
        F::Storage,
        bool,
    );
    type Output = (A, B, C, D, E, F, bool);

    fn apply(
        dest: &mut Self::Storage,
        data: PassBy<ByteSliceSignal>,
        parameters: &Parameters,
    ) -> Result<(), ()> {
        let data = core::str::from_utf8(data).or(Err(()))?;
        let data: Object = json::from_str(data).or(Err(()))?;
        let v1 = A::from_json_object(&data, &parameters.select_data[0].1);
        let v2 = B::from_json_object(&data, &parameters.select_data[1].1);
        let v3 = C::from_json_object(&data, &parameters.select_data[2].1);
        let v4 = D::from_json_object(&data, &parameters.select_data[3].1);
        let v5 = E::from_json_object(&data, &parameters.select_data[4].1);
        let v6 = F::from_json_object(&data, &parameters.select_data[5].1);
        if let (Ok(v1), Ok(v2), Ok(v3), Ok(v4), Ok(v5), Ok(v6)) = (v1, v2, v3, v4, v5, v6) {
            *dest = (v1, v2, v3, v4, v5, v6, true);
            Ok(())
        } else {
            dest.6 = false;
            Err(())
        }
    }
    fn default_storage() -> Self::Storage {
        (
            A::default_storage(),
            B::default_storage(),
            C::default_storage(),
            D::default_storage(),
            E::default_storage(),
            F::default_storage(),
            false,
        )
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
}

// Impl for tuple of seven values
impl<
        A: Deserialize,
        B: Deserialize,
        C: Deserialize,
        D: Deserialize,
        E: Deserialize,
        F: Deserialize,
        G: Deserialize,
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
    type Output = (A, B, C, D, E, F, G, bool);

    fn apply(
        dest: &mut Self::Storage,
        data: PassBy<ByteSliceSignal>,
        parameters: &Parameters,
    ) -> Result<(), ()> {
        let data = core::str::from_utf8(data).or(Err(()))?;
        let data: Object = json::from_str(data).or(Err(()))?;
        let v1 = A::from_json_object(&data, &parameters.select_data[0].1);
        let v2 = B::from_json_object(&data, &parameters.select_data[1].1);
        let v3 = C::from_json_object(&data, &parameters.select_data[2].1);
        let v4 = D::from_json_object(&data, &parameters.select_data[3].1);
        let v5 = E::from_json_object(&data, &parameters.select_data[4].1);
        let v6 = F::from_json_object(&data, &parameters.select_data[5].1);
        let v7 = G::from_json_object(&data, &parameters.select_data[6].1);
        if let (Ok(v1), Ok(v2), Ok(v3), Ok(v4), Ok(v5), Ok(v6), Ok(v7)) =
            (v1, v2, v3, v4, v5, v6, v7)
        {
            *dest = (v1, v2, v3, v4, v5, v6, v7, true);
            Ok(())
        } else {
            dest.7 = false;
            Err(())
        }
    }

    fn default_storage() -> Self::Storage {
        (
            A::default_storage(),
            B::default_storage(),
            C::default_storage(),
            D::default_storage(),
            E::default_storage(),
            F::default_storage(),
            G::default_storage(),
            false,
        )
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
}

#[cfg(test)]
mod tests {
    use crate::testing::StubContext;

    use super::*;

    #[test]
    fn test_reads_scalar_data_if_no_selectors() {
        let ctxt = StubContext::default();
        let input = br#"1.2"#;
        let params = Parameters::new(&[], 0.0);
        let mut block = JsonLoadBlock::<f64>::default();
        let res = block.process(&params, &ctxt, input);
        assert_eq!(res, (1.2, true));
        assert_eq!(block.data, vec![OldBlockData::from_scalar(1.2)]);
        assert!(block.is_valid(ctxt.time().as_secs_f64()).any());
    }

    #[test]
    fn test_reads_object_data_if_has_selectors() {
        let ctxt = StubContext::default();
        let input = br#" {"foo": 99.0, "bar": "hello", "baz": [1.0, 2.0], "buzz": [[1.0, 0.0],[0.0, 1.0]]} "#;
        let params = Parameters::new(
            &[
                "Scalar:foo".into(),
                "BytesArray:bar".into(),
                "Scalar:baz".into(),
                "Scalar:buzz".into(),
            ],
            1000.0,
        );
        let mut block =
            JsonLoadBlock::<(f64, ByteSliceSignal, Matrix<2, 1, f64>, Matrix<2, 2, f64>)>::default(
            );

        let res = block.process(&params, &ctxt, input);
        let expected = (
            99.0,
            b"hello".as_slice(),
            &Matrix { data: [[1.0, 2.0]] },
            &Matrix {
                data: [[1.0, 0.0], [0.0, 1.0]],
            },
            true,
        );

        assert_eq!(res, expected);
        assert_eq!(
            block.data,
            vec![
                <OldBlockData as FromPass<f64>>::from_pass(expected.0),
                <OldBlockData as FromPass<ByteSliceSignal>>::from_pass(expected.1),
                <OldBlockData as FromPass<Matrix<2, 1, f64>>>::from_pass(expected.2),
                <OldBlockData as FromPass<Matrix<2, 2, f64>>>::from_pass(expected.3),
            ]
        );
        assert!(block.is_valid(ctxt.time().as_secs_f64()).any());
    }

    #[test]
    fn test_reads_invalid_json_input_without_panicking() {
        let ctxt = StubContext::default();
        let input = b"invalid_json";
        let params = Parameters::new(&[], 1000.0);
        let mut block = JsonLoadBlock::<f64>::default();
        let res = block.process(&params, &ctxt, input);
        assert_eq!(res, (0.0, false));
        assert_eq!(block.data, vec![OldBlockData::from_scalar(0.0)]);
        assert!(!block.is_valid(ctxt.time().as_secs_f64()).any());
    }

    #[test]
    fn test_reads_non_existing_key_in_selector() {
        let ctxt = StubContext::default();
        let input = br#"{"foo": 99.0, "bar": "hello"}"#;
        let params = Parameters::new(&["Scalar:non_existing_key".into()], 1000.0);
        let mut block = JsonLoadBlock::<f64>::default();
        let res = block.process(&params, &ctxt, input);
        assert_eq!(res, (0.0, false));
        assert_eq!(block.data, vec![OldBlockData::from_scalar(0.0)]);
        assert!(!block.is_valid(ctxt.time().as_secs_f64()).any());
    }

    #[test]
    fn test_reads_empty_input() {
        let ctxt = StubContext::default();
        let input = b"";
        let params = Parameters::new(&[], 1000.0);
        let mut block = JsonLoadBlock::<f64>::default();
        let res = block.process(&params, &ctxt, input);
        assert_eq!(res, (0.0, false));
        assert_eq!(block.data, vec![OldBlockData::from_scalar(0.0)]);
        assert!(!block.is_valid(ctxt.time().as_secs_f64()).any());
    }

    #[test]
    fn test_reads_numeric_array() {
        let ctxt = StubContext::default();
        let input = br#"[1.0, 2.0, 3.0]"#;
        let params = Parameters::new(&[], 1000.0);
        let mut block = JsonLoadBlock::<Matrix<1, 3, f64>>::default();
        let res = block.process(&params, &ctxt, input);
        let expected = &Matrix {
            data: [[1.0], [2.0], [3.0]],
        };
        assert_eq!(res, (expected, true));
        assert_eq!(
            block.data,
            vec![OldBlockData::from_matrix(&[&[1.0, 2.0, 3.0]])]
        );
        assert!(block.is_valid(ctxt.time().as_secs_f64()).any());
    }

    #[test]
    fn test_reads_empty_numeric_array() {
        let ctxt = StubContext::default();
        let input = br#"[]"#;
        let params = Parameters::new(&[], 1000.0);
        let mut block = JsonLoadBlock::<Matrix<0, 1, f64>>::default();
        let res = block.process(&params, &ctxt, input);
        let expected = &Matrix { data: [[]] };
        assert_eq!(res, (expected, true));
        assert_eq!(block.data, vec![OldBlockData::from_row_slice(0, 1, &[])]);
        assert!(block.is_valid(ctxt.time().as_secs_f64()).any());
    }

    #[test]
    fn test_reads_numeric_matrix() {
        let ctxt = StubContext::default();
        let input = br#"[[1.0, 2.0], [3.0, 4.0]]"#;
        let params = Parameters::new(&[], 1000.0);
        let mut block = JsonLoadBlock::<Matrix<2, 2, f64>>::default();
        let res = block.process(&params, &ctxt, input);
        let expected = &Matrix {
            data: [[1.0, 3.0], [2.0, 4.0]],
        };
        assert_eq!(res, (expected, true));
        assert_eq!(
            block.data,
            vec![OldBlockData::from_matrix(&[&[1.0, 2.0], &[3.0, 4.0]])]
        );
        assert!(block.is_valid(ctxt.time().as_secs_f64()).any());
    }

    #[test]
    fn test_load_single_variant() {
        let ctxt = StubContext::default();
        let input = br#"{"foo": 1.0}"#;
        let params = Parameters::new(&["Scalar:foo".into()], 1000.0);
        let mut block = JsonLoadBlock::<f64>::default();
        let res = block.process(&params, &ctxt, input);
        assert_eq!(res, (1.0, true));
        assert_eq!(block.data, vec![OldBlockData::from_scalar(1.0)]);
        assert!(block.is_valid(ctxt.time().as_secs_f64()).any());
    }

    #[test]
    fn test_load_2_tuple_variant() {
        let ctxt = StubContext::default();
        let input = br#"{"foo": 1.0, "bar": "hello"}"#;
        let params = Parameters::new(&["Scalar:foo".into(), "BytesArray:bar".into()], 1000.0);
        let mut block = JsonLoadBlock::<(f64, ByteSliceSignal)>::default();
        let res = block.process(&params, &ctxt, input);
        assert_eq!(res, (1.0, b"hello".as_slice(), true));
        assert_eq!(
            block.data,
            vec![
                OldBlockData::from_scalar(1.0),
                OldBlockData::from_bytes(b"hello"),
            ]
        );
        assert!(block.is_valid(ctxt.time().as_secs_f64()).any());
    }

    #[test]
    fn test_load_3_tuple_variant() {
        let ctxt = StubContext::default();
        let input = br#"{"foo": 1.0, "bar": "hello", "baz": [1.0, 2.0]}"#;
        let params = Parameters::new(
            &[
                "Scalar:foo".into(),
                "BytesArray:bar".into(),
                "Scalar:baz".into(),
            ],
            1000.0,
        );
        let mut block = JsonLoadBlock::<(f64, ByteSliceSignal, Matrix<1, 2, f64>)>::default();
        let res = block.process(&params, &ctxt, input);
        assert_eq!(
            res,
            (
                1.0,
                b"hello".as_slice(),
                &Matrix {
                    data: [[1.0], [2.0]]
                },
                true
            )
        );
        assert_eq!(
            block.data,
            vec![
                OldBlockData::from_scalar(1.0),
                OldBlockData::from_bytes(b"hello"),
                OldBlockData::from_matrix(&[&[1.0, 2.0]]),
            ]
        );
        assert!(block.is_valid(ctxt.time().as_secs_f64()).any());
    }

    #[test]
    fn test_load_4_tuple_variant() {
        let ctxt = StubContext::default();
        let input =
            br#"{"foo": 1.0, "bar": "hello", "baz": [1.0, 2.0], "buzz": [[1.0, 0.0],[0.0, 1.0]]}"#;
        let params = Parameters::new(
            &[
                "Scalar:foo".into(),
                "BytesArray:bar".into(),
                "Scalar:baz".into(),
                "Scalar:buzz".into(),
            ],
            1000.0,
        );
        let mut block =
            JsonLoadBlock::<(f64, ByteSliceSignal, Matrix<1, 2, f64>, Matrix<2, 2, f64>)>::default(
            );
        let res = block.process(&params, &ctxt, input);
        assert_eq!(
            res,
            (
                1.0,
                b"hello".as_slice(),
                &Matrix {
                    data: [[1.0], [2.0]]
                },
                &Matrix {
                    data: [[1.0, 0.0], [0.0, 1.0]]
                },
                true
            )
        );
        assert_eq!(
            block.data,
            vec![
                OldBlockData::from_scalar(1.0),
                OldBlockData::from_bytes(b"hello"),
                OldBlockData::from_matrix(&[&[1.0, 2.0]]),
                OldBlockData::from_matrix(&[&[1.0, 0.0], &[0.0, 1.0]]),
            ]
        );
        assert!(block.is_valid(ctxt.time().as_secs_f64()).any());
    }

    #[test]
    fn test_load_5_tuple_variant() {
        let ctxt = StubContext::default();
        let input = br#"{"foo": 1.0, "bar": "hello", "baz": [1.0, 2.0], "buzz": [[1.0, 0.0],[0.0, 1.0]], "qux": [1.0]}"#;
        let params = Parameters::new(
            &[
                "Scalar:foo".into(),
                "BytesArray:bar".into(),
                "Scalar:baz".into(),
                "Scalar:buzz".into(),
                "Scalar:qux".into(),
            ],
            1000.0,
        );
        let mut block = JsonLoadBlock::<(
            f64,
            ByteSliceSignal,
            Matrix<1, 2, f64>,
            Matrix<2, 2, f64>,
            Matrix<1, 1, f64>,
        )>::default();
        let res = block.process(&params, &ctxt, input);
        assert_eq!(
            res,
            (
                1.0,
                b"hello".as_slice(),
                &Matrix {
                    data: [[1.0], [2.0]]
                },
                &Matrix {
                    data: [[1.0, 0.0], [0.0, 1.0]]
                },
                &Matrix { data: [[1.0]] },
                true
            )
        );
        assert_eq!(
            block.data,
            vec![
                OldBlockData::from_scalar(1.0),
                OldBlockData::from_bytes(b"hello"),
                OldBlockData::from_matrix(&[&[1.0, 2.0]]),
                OldBlockData::from_matrix(&[&[1.0, 0.0], &[0.0, 1.0]]),
                OldBlockData::from_matrix(&[&[1.0]]),
            ]
        );
        assert!(block.is_valid(ctxt.time().as_secs_f64()).any());
    }

    #[test]
    fn test_load_6_tuple_variant() {
        let ctxt = StubContext::default();
        let input = br#"{"foo": 1.0, "bar": "hello", "baz": [1.0, 2.0], "buzz": [[1.0, 0.0],[0.0, 1.0]], "qux": [1.0], "quux": [2.0]}"#;
        let params = Parameters::new(
            &[
                "Scalar:foo".into(),
                "BytesArray:bar".into(),
                "Scalar:baz".into(),
                "Scalar:buzz".into(),
                "Scalar:qux".into(),
                "Scalar:quux".into(),
            ],
            1000.0,
        );
        let mut block = JsonLoadBlock::<(
            f64,
            ByteSliceSignal,
            Matrix<1, 2, f64>,
            Matrix<2, 2, f64>,
            Matrix<1, 1, f64>,
            Matrix<1, 1, f64>,
        )>::default();
        let res = block.process(&params, &ctxt, input);
        assert_eq!(
            res,
            (
                1.0,
                b"hello".as_slice(),
                &Matrix {
                    data: [[1.0], [2.0]]
                },
                &Matrix {
                    data: [[1.0, 0.0], [0.0, 1.0]]
                },
                &Matrix { data: [[1.0]] },
                &Matrix { data: [[2.0]] },
                true
            )
        );
        assert_eq!(
            block.data,
            vec![
                OldBlockData::from_scalar(1.0),
                OldBlockData::from_bytes(b"hello"),
                OldBlockData::from_matrix(&[&[1.0, 2.0]]),
                OldBlockData::from_matrix(&[&[1.0, 0.0], &[0.0, 1.0]]),
                OldBlockData::from_matrix(&[&[1.0]]),
                OldBlockData::from_matrix(&[&[2.0]]),
            ]
        );
        assert!(block.is_valid(ctxt.time().as_secs_f64()).any());
    }

    #[test]
    fn test_load_7_tuple_variant() {
        let ctxt = StubContext::default();
        let input = br#"{
        "foo": 1.0,
        "bar": "hello",
        "baz": [1.0, 2.0],
        "buzz": [[1.0, 0.0], [0.0, 1.0]],
        "qux": [1.0],
        "quux": [2.0],
        "corge": [[3.0, 4.0], [5.0, 6.0]]
    }"#;

        let params = Parameters::new(
            &[
                "Scalar:foo".into(),
                "BytesArray:bar".into(),
                "Scalar:baz".into(),
                "Scalar:buzz".into(),
                "Scalar:qux".into(),
                "Scalar:quux".into(),
                "Scalar:corge".into(),
            ],
            1000.0,
        );

        let mut block = JsonLoadBlock::<(
            f64,
            ByteSliceSignal,
            Matrix<1, 2, f64>,
            Matrix<2, 2, f64>,
            Matrix<1, 1, f64>,
            Matrix<1, 1, f64>,
            Matrix<2, 2, f64>,
        )>::default();

        let res = block.process(&params, &ctxt, input);

        let expected = (
            1.0,
            b"hello".as_slice(),
            &Matrix {
                data: [[1.0], [2.0]],
            },
            &Matrix {
                data: [[1.0, 0.0], [0.0, 1.0]],
            },
            &Matrix { data: [[1.0]] },
            &Matrix { data: [[2.0]] },
            &Matrix {
                data: [[3.0, 5.0], [4.0, 6.0]],
            },
            true,
        );

        assert_eq!(res, expected);

        assert_eq!(
            block.data,
            vec![
                OldBlockData::from_scalar(1.0),
                OldBlockData::from_bytes(b"hello"),
                OldBlockData::from_matrix(&[&[1.0, 2.0]]),
                OldBlockData::from_matrix(&[&[1.0, 0.0], &[0.0, 1.0]]),
                OldBlockData::from_matrix(&[&[1.0]]),
                OldBlockData::from_matrix(&[&[2.0]]),
                OldBlockData::from_matrix(&[&[3.0, 4.0], &[5.0, 6.0]]),
            ]
        );

        assert!(block.is_valid(ctxt.time().as_secs_f64()).any());
    }
}
