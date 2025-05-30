use crate::{nalgebra_interop::MatrixExt, IsValid};
use nalgebra::{
    allocator::Allocator, ArrayStorage, Const, DefaultAllocator, DimDiff, DimMin, DimMinimum,
    DimSub, SMatrix, SquareMatrix, ToTypenum, SVD, U1,
};
use pictorus_block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock};

use crate::traits::{Float, Scalar};

// These need to be defined as traits so we can create distinct implementations
// of MatrixInverseBlock depending on the method. In particular, nalgebra's
// type system does not implement an inverse method for non-square matrices.
// So we need to define distinct type bounds on the dimensions for standard inverse
// and SVD-based pseudo-inverse.

/// Method for selecting which type of matrix inversion to perform.
pub trait Method {}
/// Standard matrix inversion for square matrices.
pub struct Inverse;
impl Method for Inverse {}

/// Singular Value Decomposition (SVD) for pseudo-inverse of matrices.
pub struct Svd;
impl Method for Svd {}

#[derive(Debug, Clone, Default)]
pub struct Parameters {}

impl Parameters {
    pub fn new() -> Parameters {
        Parameters {}
    }
}

/// Inverts a matrix using either a standard inverse or a pseudo-inverse based on the method specified.
///
/// This can either attempt to perform a standard matrix inversion on a square matrix,
/// or a pseudo-inverse using SVD for matrices that may or may not be square.
/// The output type of the block is a tuple of (<input_type>, bool), where the
/// bool indicates whether the inversion was successful.
pub struct MatrixInverseBlock<T: Apply<M>, M: Method> {
    pub data: OldBlockData,
    store: Option<T::Output>,
    is_data_valid: bool,
}

impl<T, M> Default for MatrixInverseBlock<T, M>
where
    M: Method,
    T: Apply<M>,
    OldBlockData: FromPass<T::Output>,
{
    fn default() -> Self {
        Self {
            data: <OldBlockData as FromPass<T::Output>>::from_pass(<T::Output>::default().as_by()),
            store: None,
            is_data_valid: false,
        }
    }
}

impl<T, M> ProcessBlock for MatrixInverseBlock<T, M>
where
    M: Method,
    T: Apply<M>,
    OldBlockData: FromPass<T::Output>,
{
    type Inputs = T;
    type Output = (T::Output, bool);
    type Parameters = Parameters;

    fn process(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        input: PassBy<Self::Inputs>,
    ) -> PassBy<Self::Output> {
        let output = T::apply(input);
        let output = match output {
            Some(output) => {
                self.is_data_valid = true;
                self.store.insert(output)
            }
            None => {
                self.is_data_valid = false;
                self.store.get_or_insert(T::Output::default())
            }
        }
        .as_by();
        self.data = OldBlockData::from_pass(output);
        (output, self.is_data_valid)
    }
}

// TODO: Remove when we remove BlockData
impl<T: Apply<M>, M: Method> IsValid for MatrixInverseBlock<T, M> {
    fn is_valid(&self, _: f64) -> OldBlockData {
        OldBlockData::scalar_from_bool(self.is_data_valid)
    }
}

pub trait Apply<M: Method>: Pass {
    type Output: Pass + Default;

    fn apply(input: PassBy<Self>) -> Option<Self::Output>;
}

// Method doesn't matter for scalar
impl<S: Scalar, M: Method> Apply<M> for S {
    type Output = S;

    fn apply<'s>(input: PassBy<Self>) -> Option<Self::Output> {
        Some(input.as_by())
    }
}

// Require equal dimensions for standard inversion
impl<const N: usize, S: Float> Apply<Inverse> for Matrix<N, N, S>
where
    Const<N>: ToTypenum + DimMin<Const<N>>,
    DimMinimum<Const<N>, Const<N>>: DimSub<U1>,
    nalgebra::DefaultAllocator: Allocator<Const<N>, Const<N>, Buffer<S> = ArrayStorage<S, N, N>>
        + Allocator<Const<N>>
        + Allocator<DimDiff<DimMinimum<Const<N>, Const<N>>, U1>>
        + Allocator<DimMinimum<Const<N>, Const<N>>, Const<N>>
        + Allocator<DimMinimum<Const<N>, Const<N>>>
        + Allocator<Const<N>, DimMinimum<Const<N>, Const<N>>>,
{
    type Output = Matrix<N, N, S>;

    fn apply<'s>(input: PassBy<Self>) -> Option<Self::Output> {
        let input: nalgebra::Matrix<S, Const<N>, Const<N>, ArrayStorage<S, N, N>> =
            SquareMatrix::from_array_storage(ArrayStorage(input.data));
        input.try_inverse().map(|r| Matrix::from_view(&r.as_view()))
    }
}

impl<const NROWS: usize, const NCOLS: usize, S: Float> Apply<Svd> for Matrix<NROWS, NCOLS, S>
where
    Const<NROWS>: ToTypenum + DimMin<Const<NCOLS>>,
    DimMinimum<Const<NROWS>, Const<NCOLS>>: DimSub<U1>, // for Bidiagonal.
    DefaultAllocator: Allocator<Const<NROWS>, Const<NCOLS>, Buffer<S> = ArrayStorage<S, NROWS, NCOLS>>
        + Allocator<Const<NCOLS>>
        + Allocator<Const<NROWS>>
        + Allocator<DimDiff<DimMinimum<Const<NROWS>, Const<NCOLS>>, U1>>
        + Allocator<DimMinimum<Const<NROWS>, Const<NCOLS>>, Const<NCOLS>>
        + Allocator<Const<NROWS>, DimMinimum<Const<NROWS>, Const<NCOLS>>>
        + Allocator<DimMinimum<Const<NROWS>, Const<NCOLS>>>,
{
    type Output = Matrix<NCOLS, NROWS, S>;

    fn apply<'s>(input: PassBy<Self>) -> Option<Self::Output> {
        let input: nalgebra::Matrix<S, Const<NROWS>, Const<NCOLS>, ArrayStorage<S, NROWS, NCOLS>> =
            SMatrix::from_array_storage(ArrayStorage(input.data));
        let svd = SVD::new(input, true, true);
        svd.pseudo_inverse(S::EPSILON)
            .ok()
            .map(|r| Matrix::<NCOLS, NROWS, S>::from_view(&r.as_view()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use approx::assert_abs_diff_eq;

    #[test]
    fn test_matrix_inverse_scalar() {
        let params = Parameters::new();
        let ctxt = StubContext::default();
        let mut block = MatrixInverseBlock::<f64, Inverse>::default();
        let res = block.process(&params, &ctxt, 99.0);
        assert_eq!(res, (99.0, true));
        assert_eq!(block.data.scalar(), 99.0);
        assert!(block.is_valid(0.0).any());
    }

    #[test]
    fn test_matrix_inverse_matrix() {
        let params = Parameters::new();
        let ctxt = StubContext::default();
        let mut block = MatrixInverseBlock::<Matrix<2, 2, f64>, Inverse>::default();
        let input = Matrix {
            data: [[1.0, 2.0], [3.0, 4.0]],
        };
        let res = block.process(&params, &ctxt, &input);
        let expected = [[-2.0, 1.0], [1.5, -0.5]];
        assert_eq!(res.0.data, expected);
        assert!(res.1);

        assert_eq!(block.data.get_data().as_slice(), expected.as_flattened());
        assert!(block.is_valid(0.0).any());
    }

    #[test]
    fn test_svd_robustness_compared_to_inverse() {
        let params = Parameters::new();
        let ctxt = StubContext::default();
        let det_zero_input = Matrix {
            data: [[1.0, 2.0, 3.0], [2.0, 4.0, 6.0], [3.0, 6.0, 8.0]],
        };

        // Regular inverse method panics with an input determinant == 0.0
        let mut invert_block = MatrixInverseBlock::<Matrix<3, 3, f64>, Inverse>::default();
        let res = invert_block.process(&params, &ctxt, &det_zero_input);
        let expected = [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]];
        assert_eq!(res.0.data, expected,);
        assert!(!res.1);

        assert_eq!(
            invert_block.data.get_data().as_slice(),
            expected.as_flattened()
        );
        assert!(!invert_block.is_valid(0.0).any());

        // SVD-based pseudo-inverse method should be fine
        let mut svd_block = MatrixInverseBlock::<Matrix<3, 3, f64>, Svd>::default();
        let res = svd_block.process(&params, &ctxt, &det_zero_input);
        let expected = [
            [-0.32, -0.64, 0.60],
            [-0.64, -1.28, 1.20],
            [0.60, 1.20, -1.00],
        ];
        assert_abs_diff_eq!(
            res.0.data.as_flattened(),
            expected.as_flattened(),
            epsilon = 1e-8
        );
        assert!(res.1);

        assert_abs_diff_eq!(
            svd_block.data.get_data().as_slice(),
            expected.as_flattened(),
            epsilon = 1e-8
        );
        assert!(svd_block.is_valid(0.0).any());
    }

    #[test]
    fn test_pseudo_inverse_square_nonsingular() {
        let params = Parameters::new();
        let ctxt = StubContext::default();
        let mut block = MatrixInverseBlock::<Matrix<3, 3, f64>, Svd>::default();
        let matrix = Matrix {
            data: [[4.0, 7.0, 2.0], [1.0, 6.0, 9.0], [5.0, 3.0, 8.0]],
        };

        // From numpy
        let expected_inverse = Matrix {
            data: [
                [0.07266436, -0.17301038, 0.17647059],
                [0.12802768, 0.07612457, -0.11764706],
                [-0.09342561, 0.07958478, 0.05882353],
            ],
        };

        let res = block.process(&params, &ctxt, &matrix);
        assert_abs_diff_eq!(
            res.0.data.as_flattened(),
            expected_inverse.data.as_flattened(),
            epsilon = 1e-8
        );
        assert!(res.1);
        assert!(block.is_valid(0.0).any());
    }

    #[test]
    fn test_pseudo_inverse_nonsquare() {
        let params = Parameters::new();
        let ctxt = StubContext::default();
        let mut block = MatrixInverseBlock::<Matrix<2, 3, f64>, Svd>::default();
        let matrix = Matrix {
            data: [[1.0, 4.0], [2.0, 5.0], [3.0, 6.0]],
        };

        // From numpy
        let expected_inverse = Matrix {
            data: [
                [-0.94444444, -0.11111111, 0.72222222],
                [0.44444444, 0.11111111, -0.22222222],
            ],
        };

        let res = block.process(&params, &ctxt, &matrix);
        assert_abs_diff_eq!(
            res.0.data.as_flattened(),
            expected_inverse.data.as_flattened(),
            epsilon = 1e-8
        );
        assert!(res.1);
        assert!(block.is_valid(0.0).any());
    }
}
