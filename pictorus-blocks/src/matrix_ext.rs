//! Traits for inter-operation with nalgebra

// NOTE this only supports nalgebra 0.33.x
use nalgebra::{Const, Dim, MatrixView, MatrixViewMut};
use pictorus_traits::Matrix;

pub trait MatrixNalgebraExt {
    type NROWS: Dim;
    type NCOLS: Dim;
    type Elem;

    fn as_view(&self) -> MatrixView<'_, Self::Elem, Self::NROWS, Self::NCOLS>;
    fn as_view_mut(&mut self) -> MatrixViewMut<'_, Self::Elem, Self::NROWS, Self::NCOLS>;
    fn from_view(view: &MatrixView<Self::Elem, Self::NROWS, Self::NCOLS>) -> Self;
}

impl<T, const NROWS: usize, const NCOLS: usize> MatrixNalgebraExt for Matrix<NROWS, NCOLS, T>
where
    T: pictorus_traits::Scalar + nalgebra::Scalar,
{
    type NROWS = Const<NROWS>;
    type NCOLS = Const<NCOLS>;
    type Elem = T;

    fn as_view(&self) -> MatrixView<'_, Self::Elem, Self::NROWS, Self::NCOLS> {
        MatrixView::<Self::Elem, Self::NROWS, Self::NCOLS>::from_slice(self.data.as_flattened())
    }

    fn as_view_mut(&mut self) -> MatrixViewMut<'_, Self::Elem, Self::NROWS, Self::NCOLS> {
        MatrixViewMut::<Self::Elem, Self::NROWS, Self::NCOLS>::from_slice(
            self.data.as_flattened_mut(),
        )
    }

    fn from_view(view: &MatrixView<Self::Elem, Self::NROWS, Self::NCOLS>) -> Self {
        let mut m = pictorus_traits::Matrix::<NROWS, NCOLS, T>::zeroed();
        m.as_view_mut().copy_from(view);
        m
    }
}

pub trait MatrixExt {
    type Elem;
    fn from_flat_array(rows: usize, cols: usize, data: &[Self::Elem]) -> Self;
    fn nrows(&self) -> usize;
    fn ncols(&self) -> usize;
    fn as_col_slice(&self) -> &[Self::Elem];
}

impl<const NROWS: usize, const NCOLS: usize, T: pictorus_traits::Scalar> MatrixExt
    for Matrix<NROWS, NCOLS, T>
{
    type Elem = T;

    fn from_flat_array(rows: usize, cols: usize, data: &[Self::Elem]) -> Self {
        // This accepts data as row-major, but pictorus_traits::Matrix is column-major, so we need to transpose the data
        let mut m = Self::zeroed();
        for r in 0..rows {
            for c in 0..cols {
                m.data[c][r] = data[r * cols + c];
            }
        }
        m
    }

    fn nrows(&self) -> usize {
        NROWS
    }

    fn ncols(&self) -> usize {
        NCOLS
    }

    fn as_col_slice(&self) -> &[Self::Elem] {
        self.data.as_flattened()
    }
}
