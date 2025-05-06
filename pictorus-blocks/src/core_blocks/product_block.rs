use block_data::{BlockData as OldBlockData, FromPass};
use core::marker::PhantomData;
use pictorus_traits::{Pass, PassBy, ProcessBlock};

// This Block is essentially two blocks hiding in a trench coat; Matrix Multiplication and Component Wise Multiplication.
// Functionality for each has been broken out into separate modules to keep file sizes in check.
mod component;
use component::ApplyComponentWise;
mod matrix;
use matrix::{ApplyMatMul, ParametersMatrixMult};

/// This block can be used to get the product of all of its input signals.
/// The product can be calculated in two ways:
/// - ComponentWise: Accepts Scalars, Same Size Matrices, or Scalars and Same Size Matrices
/// - MatrixMultiply: Accepts all matrices, using standard matrix multiplication sizing rules (i.e. (A, B) * (B, C) = (A, C))
pub struct ProductBlock<T: Apply<M>, M: ProductMethod> {
    _method: PhantomData<M>,
    store: Option<T::Output>,
    pub data: OldBlockData,
}

impl<T: Apply<M>, M: ProductMethod> Default for ProductBlock<T, M>
where
    OldBlockData: FromPass<T::Output>,
{
    fn default() -> Self {
        Self {
            _method: PhantomData,
            store: None,
            data: <OldBlockData as FromPass<T::Output>>::from_pass(T::Output::default().as_by()),
        }
    }
}

impl<T: Apply<M>, M: ProductMethod> ProcessBlock for ProductBlock<T, M>
where
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
        let output = T::apply(&mut self.store, parameters, inputs);
        self.data = OldBlockData::from_pass(output);
        output
    }
}

/// This trait is what allows us to use either ComponentWise or MatrixMultiply
/// as the method for the ProductBlock.
pub trait Apply<M: ProductMethod>: Pass {
    type Output: Pass + Default;
    type Parameters;
    fn apply<'a>(
        buffer: &'a mut Option<Self::Output>,
        parameters: &Self::Parameters,
        inputs: PassBy<Self>,
    ) -> PassBy<'a, Self::Output>;
}

impl<T: ApplyMatMul> Apply<MatrixMultiply> for T {
    type Output = T::Output;
    type Parameters = ParametersMatrixMult;
    fn apply<'a>(
        buffer: &'a mut Option<Self::Output>,
        _parameters: &Self::Parameters,
        inputs: PassBy<Self>,
    ) -> PassBy<'a, Self::Output> {
        T::mat_mul(inputs, buffer)
    }
}

impl<T: ApplyComponentWise> Apply<ComponentWise> for T {
    type Output = T::Output;
    type Parameters = T::Parameters;
    fn apply<'a>(
        buffer: &'a mut Option<Self::Output>,
        parameters: &Self::Parameters,
        inputs: PassBy<Self>,
    ) -> PassBy<'a, Self::Output> {
        *buffer = None; // Reset Dest as None
        T::apply(inputs, parameters, buffer)
    }
}

/// This trait is used as a marker for the two different methods of product calculation.
pub trait ProductMethod {}
pub struct ComponentWise;
impl ProductMethod for ComponentWise {}
pub struct MatrixMultiply;
impl ProductMethod for MatrixMultiply {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use component::ParametersComponentWise;
    use pictorus_traits::Matrix;

    #[test]
    fn test_component_wise_scalar() {
        let context = StubContext::default();

        // Scalars only
        let mut block = ProductBlock::<(f64, f64), ComponentWise>::default();
        let parameters =
            <ProductBlock<(f64, f64), ComponentWise> as ProcessBlock>::Parameters::new([1.0, 1.0]);
        let output = block.process(&parameters, &context, (11.0, 2.0));

        assert_eq!(output, 22.0);
        assert_eq!(block.data.scalar(), 22.0);

        let mut block = ProductBlock::<(f32, f32, f32, f32, f32), ComponentWise>::default();
        let parameters = ParametersComponentWise::new([1.0, 1.0, 1.0, -1.0, 1.0]);
        let output = block.process(&parameters, &context, (11.0, 2.0, 3.0, 4.0, 5.0));

        assert_eq!(output, 82.5);
        assert_eq!(block.data.scalar(), 82.5);
    }
    #[test]
    fn test_component_wise_scalar_matrix_mixed() {
        let context = StubContext::default();

        // Mixed Scalars and Matrices
        let mut block = ProductBlock::<(f64, Matrix<2, 2, f64>, f64), ComponentWise>::default();
        let parameters = ParametersComponentWise::new([1.0, 1.0, 1.0]);
        let output = block.process(
            &parameters,
            &context,
            (
                11.0,
                &Matrix {
                    data: [[1.0, 2.0], [3.0, 4.0]],
                },
                1.0,
            ),
        );
        let expected = Matrix {
            data: [[11.0, 22.0], [33.0, 44.0]],
        };

        assert_eq!(output, &expected);
        assert_eq!(
            block.data,
            <OldBlockData as FromPass<Matrix<2, 2, f64>>>::from_pass(&expected)
        );
    }

    #[test]
    fn test_component_wise_matrix() {
        let context = StubContext::default();

        // Matrices only
        let mut block =
            ProductBlock::<(Matrix<2, 2, f64>, Matrix<2, 2, f64>), ComponentWise>::default();
        let parameters =
            <ProductBlock<(Matrix<2, 2, f64>, Matrix<2, 2, f64>), ComponentWise> as ProcessBlock>::Parameters::new([1.0, 1.0]);
        let output = block.process(
            &parameters,
            &context,
            (
                &Matrix {
                    data: [[1.0, 2.0], [3.0, 4.0]],
                },
                &Matrix {
                    data: [[5.0, 6.0], [7.0, 8.0]],
                },
            ),
        );
        let expected = Matrix {
            data: [[5.0, 12.0], [21.0, 32.0]],
        };

        assert_eq!(output, &expected);
        assert_eq!(
            block.data,
            <OldBlockData as FromPass<Matrix<2, 2, f64>>>::from_pass(&expected)
        );
    }

    #[test]
    fn test_matrix_mult() {
        let context = StubContext::default();
        let p = ParametersMatrixMult {};

        let mut block =
            ProductBlock::<(Matrix<2, 2, f64>, Matrix<2, 2, f64>), MatrixMultiply>::default();
        let output = block.process(
            &p,
            &context,
            (
                &Matrix {
                    data: [[1.0, 3.0], [2.0, 4.0]],
                },
                &Matrix {
                    data: [[5.0, 7.0], [6.0, 8.0]],
                },
            ),
        );
        let expected = Matrix {
            data: [[19.0, 43.0], [22.0, 50.0]],
        };

        assert_eq!(output, &expected);
        assert_eq!(
            block.data,
            <OldBlockData as FromPass<Matrix<2, 2, f64>>>::from_pass(&expected)
        );

        let mut block = ProductBlock::<
            (Matrix<4, 2, f64>, Matrix<2, 3, f64>, Matrix<3, 2, f64>),
            MatrixMultiply,
        >::default();
        let output = block.process(
            &p,
            &context,
            (
                &Matrix {
                    data: [[1.0, 2.0, 3.0, 4.0], [5.0, 6.0, 7.0, 8.0]],
                },
                &Matrix {
                    data: [[5.0, 6.0], [7.0, 8.0], [9.0, 10.0]],
                },
                &Matrix {
                    data: [[42.0, 11.0, 12.0], [1337.0, 12.0, -4.0]],
                },
            ),
        );
        let expected = Matrix {
            data: [
                [2695.0, 3550.0, 4405.0, 5260.0],
                [47123.0, 61934.0, 76745.0, 91556.0],
            ],
        };

        assert_eq!(output, &expected);
        assert_eq!(
            block.data,
            <OldBlockData as FromPass<Matrix<4, 2, f64>>>::from_pass(&expected)
        );
    }
}
