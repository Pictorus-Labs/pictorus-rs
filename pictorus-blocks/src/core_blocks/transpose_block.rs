use crate::matrix_ext::MatrixNalgebraExt;
use pictorus_block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock};

use crate::traits::Scalar;

pub struct Parameters {}

impl Parameters {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for Parameters {
    fn default() -> Self {
        Self::new()
    }
}
/// Outputs the transpose of the input signal.
///
/// For scalar inputs this is just a pass-through
pub struct TransposeBlock<T: Apply> {
    pub data: OldBlockData,
    store: T::Output,
}

impl<T: Apply> Default for TransposeBlock<T>
where
    OldBlockData: FromPass<T::Output>,
{
    fn default() -> Self {
        Self {
            data: <OldBlockData as FromPass<T::Output>>::from_pass(<T::Output>::default().as_by()),
            store: T::Output::default(),
        }
    }
}

impl<T: Apply> ProcessBlock for TransposeBlock<T>
where
    OldBlockData: FromPass<T::Output>,
{
    type Inputs = T;
    type Output = T::Output;
    type Parameters = Parameters;

    fn process(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        input: PassBy<Self::Inputs>,
    ) -> PassBy<'_, Self::Output> {
        let output = T::apply(&mut self.store, input);
        self.data = OldBlockData::from_pass(output);
        output
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        self.store.as_by()
    }
}

pub trait Apply: Pass {
    type Output: Pass + Default;

    fn apply<'s>(store: &'s mut Self::Output, input: PassBy<Self>) -> PassBy<'s, Self::Output>;
}

impl<S: Scalar> Apply for S {
    type Output = S;

    fn apply<'s>(store: &'s mut Self::Output, input: PassBy<Self>) -> PassBy<'s, Self::Output> {
        *store = input;
        store.as_by()
    }
}

impl<const NROWS: usize, const NCOLS: usize, S: Scalar> Apply for Matrix<NROWS, NCOLS, S> {
    type Output = Matrix<NCOLS, NROWS, S>;

    fn apply<'s>(store: &'s mut Self::Output, input: PassBy<Self>) -> PassBy<'s, Self::Output> {
        let input = input.as_view();
        let transposed = input.transpose();
        *store = Matrix::from_view(&transposed.as_view());
        store
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::StubContext;

    use super::*;

    #[test]
    fn test_transpose_default_buffer_no_panic() {
        let block = TransposeBlock::<f64>::default();
        assert_eq!(block.buffer(), 0.0);

        let block = TransposeBlock::<Matrix<3, 2, f64>>::default();
        assert_eq!(block.buffer(), &Matrix::<2, 3, f64>::zeroed());
    }

    #[test]
    fn test_tranpose_scalar_input() {
        let ctxt = StubContext::default();
        let params = Parameters::default();
        let mut transpose_block = TransposeBlock::<f64>::default();

        let output = transpose_block.process(&params, &ctxt, 1.0);
        assert_eq!(output, 1.0);
        assert_eq!(transpose_block.data.scalar(), 1.0);
        assert_eq!(transpose_block.buffer(), output);

        let output = transpose_block.process(&params, &ctxt, 42.0);
        assert_eq!(output, 42.0);
        assert_eq!(transpose_block.data.scalar(), 42.0);
    }

    #[test]
    fn test_tranpose_matrix_input() {
        let ctxt = StubContext::default();
        let params = Parameters::default();
        let mut transpose_block = TransposeBlock::<Matrix<3, 2, f64>>::default();

        let input = Matrix {
            data: [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]],
        };
        let expected = Matrix {
            data: [[1.0, 4.0], [2.0, 5.0], [3.0, 6.0]],
        };
        let output = transpose_block.process(&params, &ctxt, &input);
        assert_eq!(output.data, expected.data);
        assert_eq!(
            transpose_block.data.get_data().as_slice(),
            expected.data.as_flattened()
        );
    }
}
