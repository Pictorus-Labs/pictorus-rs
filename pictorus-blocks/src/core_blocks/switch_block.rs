extern crate alloc;
use alloc::vec::Vec;
use block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{ByteSliceSignal, Matrix, Pass, PassBy, ProcessBlock};

use crate::traits::{CopyInto, DefaultStorage, Scalar};

/// Block that allows switching between multiple signals based on a scalar condition.
/// The condition is the first input, and the rest are the signals to switch between.
/// The block will output the signal that corresponds to the index of the `cases`` parameter
/// that matches the condition input. If no matches are found, it will output the last input.
/// For example:
/// ```
/// use core::time::Duration;
/// use pictorus_blocks::SwitchBlock;
/// use pictorus_traits::ProcessBlock;
/// use block_data::BlockData as OldBlockData;
/// use pictorus_traits::Context;
///
/// #[derive(Default)]
/// struct StubContext {}
///
/// impl Context for StubContext {
///     fn time(&self) -> Duration {
///         Duration::from_secs(0)
///     }
///
///     fn timestep(&self) -> Option<Duration> {
///         None
///     }
///
///     fn fundamental_timestep(&self) -> Duration {
///         Duration::from_millis(100)
///     }
/// }
///
/// let ctxt = StubContext::default();
/// let mut block = SwitchBlock::<(f64, f64, f64)>::default();
/// // If condition is 0, output the signal at index 0
/// // If condition is 1, output the signal at index 1
/// // If condition is anything else, output the signal at index 1
/// let cases = OldBlockData::from_vector(&[0.0, 1.0]);
/// let parameters = <SwitchBlock<(f64, f64, f64)> as ProcessBlock>::Parameters::new(&cases);
/// // Here we have a condition of 0.0, and inputs of [1.0, 2.0]
/// // Since condition matches case 0, the output will be 1.0
/// let input = (0.0, 1.0, 2.0);
/// let output = block.process(&parameters, &ctxt, input);
/// assert_eq!(output, 1.0);
///
pub struct SwitchBlock<T: Apply>
where
    T::Output: DefaultStorage,
    OldBlockData: FromPass<T::Output>,
{
    pub data: OldBlockData,
    buffer: <T::Output as DefaultStorage>::Storage,
}

impl<T: Apply> Default for SwitchBlock<T>
where
    T::Output: DefaultStorage,
    OldBlockData: FromPass<T::Output>,
{
    fn default() -> Self {
        Self {
            data: <OldBlockData as FromPass<T::Output>>::from_pass(T::Output::from_storage(
                &T::Output::default_storage(),
            )),
            buffer: T::Output::default_storage(),
        }
    }
}

impl<T: Apply> ProcessBlock for SwitchBlock<T>
where
    T::Output: DefaultStorage,
    OldBlockData: FromPass<T::Output>,
{
    type Inputs = T;
    type Output = T::Output;
    type Parameters = T::Parameters;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        T::apply(inputs, parameters, &mut self.buffer);
        let res = T::Output::from_storage(&self.buffer);
        self.data = <OldBlockData as FromPass<T::Output>>::from_pass(res);
        res
    }
}

/// Parameters for the SwitchBlock
pub struct Parameters<C: Scalar, const N: usize> {
    /// The cases to compare the input condition against
    /// The cases array must be exactly the same length as the number of inputs
    /// The last case is the default value
    pub cases: [C; N],
}

// TODO: This is currently only implemented for f64 and is constructed from OldBlockData.
// In the future this should either accept an array of [C; N] or a &[C]
impl<const N: usize> Parameters<f64, N> {
    pub fn new(cases: &OldBlockData) -> Self {
        assert!(cases.len() == N, "Invalid number of switch cases");

        let mut case_arr: [f64; N] = [0.0; N];
        for (idx, case) in cases.iter().enumerate() {
            case_arr[idx] = *case;
        }
        Self { cases: case_arr }
    }
}

pub trait ApplyInto<C: Scalar, const N: usize>: Pass + DefaultStorage {
    fn apply_into(
        condition: C,
        cases: &[C; N],
        inputs: &[PassBy<Self>; N],
        dest: &mut Self::Storage,
    );
}

impl<C: Scalar, const N: usize> ApplyInto<C, N> for C {
    fn apply_into(condition: C, cases: &[C; N], inputs: &[PassBy<C>; N], dest: &mut C) {
        for (idx, case) in cases.iter().enumerate() {
            if condition == *case {
                let res = inputs[idx];
                *dest = res;
                return;
            }
        }
        let res = inputs[inputs.len() - 1];
        *dest = res;
    }
}

impl<C: Scalar, const NROWS: usize, const NCOLS: usize, const N: usize> ApplyInto<C, N>
    for Matrix<NROWS, NCOLS, C>
{
    fn apply_into(
        condition: C,
        cases: &[C; N],
        inputs: &[PassBy<Matrix<NROWS, NCOLS, C>>; N],
        dest: &mut Matrix<NROWS, NCOLS, C>,
    ) {
        for (idx, case) in cases.iter().enumerate() {
            if condition == *case {
                let res = inputs[idx];
                Matrix::copy_into(res, dest);
                return;
            }
        }
        let res = inputs[inputs.len() - 1];
        Matrix::copy_into(res, dest);
    }
}

impl<C: Scalar, const N: usize> ApplyInto<C, N> for ByteSliceSignal {
    fn apply_into(
        condition: C,
        cases: &[C; N],
        inputs: &[PassBy<ByteSliceSignal>; N],
        dest: &mut Vec<u8>,
    ) {
        for (idx, case) in cases.iter().enumerate() {
            if condition == *case {
                let res = inputs[idx];
                dest.clear();
                dest.extend_from_slice(res);
                return;
            }
        }
        let res = inputs[inputs.len() - 1];
        dest.copy_from_slice(res);
    }
}

pub trait Apply: Pass {
    type Parameters;
    type Output: Pass + DefaultStorage;

    fn apply(
        input: PassBy<Self>,
        params: &Self::Parameters,
        buffer: &mut <Self::Output as DefaultStorage>::Storage,
    );
}

// SwitchBlock requires at least 3 inputs. The first is the condition,
// the rest are inputs to maybe pass through

// 1 condition + 2 inputs
impl<C: Scalar, T: Pass + DefaultStorage + ApplyInto<C, 2>> Apply for (C, T, T) {
    type Output = T;
    type Parameters = Parameters<C, 2>;

    fn apply(
        input: PassBy<Self>,
        params: &Self::Parameters,
        buffer: &mut <Self::Output as DefaultStorage>::Storage,
    ) {
        let condition = input.0;
        T::apply_into(condition, &params.cases, &[input.1, input.2], buffer);
    }
}

// 1 condition + 3 inputs
impl<C: Scalar, T: Pass + DefaultStorage + ApplyInto<C, 3>> Apply for (C, T, T, T) {
    type Output = T;
    type Parameters = Parameters<C, 3>;

    fn apply(
        input: PassBy<Self>,
        params: &Self::Parameters,
        buffer: &mut <Self::Output as DefaultStorage>::Storage,
    ) {
        let condition = input.0;
        T::apply_into(
            condition,
            &params.cases,
            &[input.1, input.2, input.3],
            buffer,
        );
    }
}

// 1 condition + 4 inputs
impl<C: Scalar, T: Pass + DefaultStorage + ApplyInto<C, 4>> Apply for (C, T, T, T, T) {
    type Output = T;
    type Parameters = Parameters<C, 4>;

    fn apply(
        input: PassBy<Self>,
        params: &Self::Parameters,
        buffer: &mut <Self::Output as DefaultStorage>::Storage,
    ) {
        let condition = input.0;
        T::apply_into(
            condition,
            &params.cases,
            &[input.1, input.2, input.3, input.4],
            buffer,
        );
    }
}

// 1 condition + 5 inputs
impl<C: Scalar, T: Pass + DefaultStorage + ApplyInto<C, 5>> Apply for (C, T, T, T, T, T) {
    type Output = T;
    type Parameters = Parameters<C, 5>;

    fn apply(
        input: PassBy<Self>,
        params: &Self::Parameters,
        buffer: &mut <Self::Output as DefaultStorage>::Storage,
    ) {
        let condition = input.0;
        T::apply_into(
            condition,
            &params.cases,
            &[input.1, input.2, input.3, input.4, input.5],
            buffer,
        );
    }
}

// 1 condition + 6 inputs
impl<C: Scalar, T: Pass + DefaultStorage + ApplyInto<C, 6>> Apply for (C, T, T, T, T, T, T) {
    type Output = T;
    type Parameters = Parameters<C, 6>;

    fn apply(
        input: PassBy<Self>,
        params: &Self::Parameters,
        buffer: &mut <Self::Output as DefaultStorage>::Storage,
    ) {
        let condition = input.0;
        T::apply_into(
            condition,
            &params.cases,
            &[input.1, input.2, input.3, input.4, input.5, input.6],
            buffer,
        );
    }
}

// 1 condition + 7 inputs
impl<C: Scalar, T: Pass + DefaultStorage + ApplyInto<C, 7>> Apply for (C, T, T, T, T, T, T, T) {
    type Output = T;
    type Parameters = Parameters<C, 7>;

    fn apply(
        input: PassBy<Self>,
        params: &Self::Parameters,
        buffer: &mut <Self::Output as DefaultStorage>::Storage,
    ) {
        let condition = input.0;
        T::apply_into(
            condition,
            &params.cases,
            &[
                input.1, input.2, input.3, input.4, input.5, input.6, input.7,
            ],
            buffer,
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::traits::MatrixOps;

    use super::*;
    use crate::testing::StubContext;

    #[test]
    fn test_switch_block_2_scalars() {
        let ctxt = StubContext::default();

        let mut block = SwitchBlock::<(f64, f64, f64)>::default();
        let parameters = Parameters::new(&OldBlockData::from_vector(&[0.0, 1.0]));

        let input = (0.0, 1.0, 2.0);
        let output = block.process(&parameters, &ctxt, input);
        assert_eq!(output, 1.0);
        assert_eq!(block.data.scalar(), 1.0);
    }

    #[test]
    fn test_switch_block_7_scalars() {
        let ctxt = StubContext::default();

        let mut block = SwitchBlock::<(f64, f64, f64, f64, f64, f64, f64, f64)>::default();
        let parameters = Parameters::new(&OldBlockData::from_vector(&[
            0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0,
        ]));

        let input = (6.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0);
        let output = block.process(&parameters, &ctxt, input);
        assert_eq!(output, 7.0);
        assert_eq!(block.data.scalar(), 7.0);
    }

    #[test]
    fn test_switch_block_scalar_default() {
        let ctxt = StubContext::default();

        let mut block = SwitchBlock::<(f64, f64, f64)>::default();
        let parameters = Parameters::new(&OldBlockData::from_vector(&[0.0, 1.0]));

        // Should use the last value by default
        let input = (1.2345, 1.0, 2.0);
        let output = block.process(&parameters, &ctxt, input);
        assert_eq!(output, 2.0);
        assert_eq!(block.data.scalar(), 2.0);
    }

    #[test]
    fn test_switch_block_2_matrices() {
        let ctxt = StubContext::default();

        let mut block = SwitchBlock::<(f64, Matrix<3, 3, f64>, Matrix<3, 3, f64>)>::default();
        let parameters = Parameters::new(&OldBlockData::from_vector(&[0.0, 1.0]));

        let input = (0.0, &Matrix::from_element(1.0), &Matrix::from_element(2.0));
        let output = block.process(&parameters, &ctxt, input);
        let expected = Matrix::from_element(1.0);
        assert_eq!(output, &expected);
        assert_eq!(
            block.data.get_data().as_slice(),
            expected.data.as_flattened()
        );
    }

    #[test]
    fn test_switch_block_7_matrices() {
        let ctxt = StubContext::default();

        let mut block = SwitchBlock::<(
            f64,
            Matrix<3, 3, f64>,
            Matrix<3, 3, f64>,
            Matrix<3, 3, f64>,
            Matrix<3, 3, f64>,
            Matrix<3, 3, f64>,
            Matrix<3, 3, f64>,
            Matrix<3, 3, f64>,
        )>::default();
        let parameters = Parameters::new(&OldBlockData::from_vector(&[
            0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0,
        ]));

        let input = (
            6.0,
            &Matrix::from_element(1.0),
            &Matrix::from_element(2.0),
            &Matrix::from_element(3.0),
            &Matrix::from_element(4.0),
            &Matrix::from_element(5.0),
            &Matrix::from_element(6.0),
            &Matrix::from_element(7.0),
        );
        let output = block.process(&parameters, &ctxt, input);
        let expected = Matrix::from_element(7.0);
        assert_eq!(output, &expected);
        assert_eq!(
            block.data.get_data().as_slice(),
            expected.data.as_flattened()
        );
    }

    #[test]
    fn test_switch_block_matrix_default() {
        let ctxt = StubContext::default();

        let mut block = SwitchBlock::<(f64, Matrix<3, 3, f64>, Matrix<3, 3, f64>)>::default();
        let parameters = Parameters::new(&OldBlockData::from_vector(&[0.0, 1.0]));

        // Should use the last value by default
        let input = (
            1.2345,
            &Matrix::from_element(1.0),
            &Matrix::from_element(2.0),
        );
        let output = block.process(&parameters, &ctxt, input);
        let expected = Matrix::from_element(2.0);
        assert_eq!(output, &expected);
        assert_eq!(
            block.data.get_data().as_slice(),
            expected.data.as_flattened()
        );
    }

    #[test]
    fn test_switch_block_2_bytes() {
        let ctxt = StubContext::default();

        let mut block = SwitchBlock::<(f64, ByteSliceSignal, ByteSliceSignal)>::default();
        let parameters = Parameters::new(&OldBlockData::from_vector(&[0.0, 1.0]));

        let input = (0.0, b"foo".as_slice(), b"bar".as_slice());
        let output = block.process(&parameters, &ctxt, input);
        assert_eq!(output, b"foo");
        assert_eq!(block.data.raw_string().as_bytes(), b"foo".as_slice());
    }

    #[test]
    fn test_switch_block_7_bytes() {
        let ctxt = StubContext::default();

        let mut block = SwitchBlock::<(
            f64,
            ByteSliceSignal,
            ByteSliceSignal,
            ByteSliceSignal,
            ByteSliceSignal,
            ByteSliceSignal,
            ByteSliceSignal,
            ByteSliceSignal,
        )>::default();
        let parameters = Parameters::new(&OldBlockData::from_vector(&[
            0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0,
        ]));

        let input = (
            6.0,
            b"foo".as_slice(),
            b"bar".as_slice(),
            b"baz".as_slice(),
            b"qux".as_slice(),
            b"quux".as_slice(),
            b"corge".as_slice(),
            b"grault".as_slice(),
        );
        let output = block.process(&parameters, &ctxt, input);
        assert_eq!(output, b"grault");
        assert_eq!(block.data.raw_string().as_bytes(), b"grault".as_slice());
    }
}
