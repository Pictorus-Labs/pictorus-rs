use crate::nalgebra_interop::MatrixExt;
use pictorus_block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{Context, Matrix, Pass, PassBy};

pub struct Parameters {
    // No parameters needed for this block
}

impl Default for Parameters {
    fn default() -> Self {
        Self::new()
    }
}

impl Parameters {
    pub fn new() -> Self {
        Self {}
    }
}

/// Performs the cross product of two 3D vectors, either 1x3 or 3x1.
pub struct CrossProductBlock<T>
where
    T: Apply + Pass + Default,
{
    pub data: OldBlockData,
    buffer: Option<T::Output>,
}

impl<T> Default for CrossProductBlock<T>
where
    T: Apply + Pass + Default,
    OldBlockData: FromPass<T::Output>,
{
    fn default() -> Self {
        Self {
            data: <OldBlockData as FromPass<T::Output>>::from_pass(T::Output::default().as_by()),
            buffer: None,
        }
    }
}

impl<T> pictorus_traits::ProcessBlock for CrossProductBlock<T>
where
    T: Pass + Apply + Default,
    OldBlockData: FromPass<T::Output>,
{
    type Inputs = T;
    type Output = T::Output;
    type Parameters = Parameters;

    fn process<'b>(
        &'b mut self,
        _parameters: &Self::Parameters,
        _context: &dyn Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        let output = T::apply(&mut self.buffer, inputs);
        self.data = OldBlockData::from_pass(output);
        output
    }
}

pub trait Apply: Pass + Sized {
    type Output: Pass + Default;

    fn apply<'s>(
        store: &'s mut Option<Self::Output>,
        inputs: PassBy<'_, Self>,
    ) -> PassBy<'s, Self::Output>;
}

macro_rules! float_matrix_impl {
    ($type:ty) => {
        impl Apply for (Matrix<1, 3, $type>, Matrix<1, 3, $type>) {
            type Output = Matrix<1, 3, $type>;

            fn apply<'s>(
                store: &'s mut Option<Self::Output>,
                inputs: PassBy<'_, Self>,
            ) -> PassBy<'s, Self::Output> {
                let cross = inputs.0.as_view().cross(&inputs.1.as_view());
                let output = store.insert(Matrix::<1, 3, $type>::from_view(&cross.as_view()));
                output
            }
        }

        impl Apply for (Matrix<3, 1, $type>, Matrix<3, 1, $type>) {
            type Output = Matrix<3, 1, $type>;

            fn apply<'s>(
                store: &'s mut Option<Self::Output>,
                inputs: PassBy<'_, Self>,
            ) -> PassBy<'s, Self::Output> {
                let cross = inputs.0.as_view().cross(&inputs.1.as_view());
                let output = store.insert(Matrix::<3, 1, $type>::from_view(&cross.as_view()));
                output
            }
        }
    };
}

float_matrix_impl!(f64);
float_matrix_impl!(f32);

#[cfg(test)]
mod tests {

    use crate::testing::StubContext;
    use pictorus_block_data::{BlockData, ToPass};
    use pictorus_traits::ProcessBlock;

    use super::*;

    #[test]
    fn test_vector_cross_f64_blockdata_1x3() {
        let context = StubContext::default();
        let p = Parameters::new();
        let mut cross_block =
            CrossProductBlock::<(Matrix<1, 3, f64>, Matrix<1, 3, f64>)>::default();
        let input1 = BlockData::new(1, 3, &[1.0, 0.0, 0.0]);
        let input2 = BlockData::new(1, 3, &[0.0, 1.0, 0.0]);
        cross_block.process(&p, &context, (&input1.to_pass(), &input2.to_pass()));

        assert_eq!(
            cross_block.data,
            BlockData::from_matrix(&[&[0.0, 0.0, 1.0]])
        );
    }

    #[test]
    fn test_vector_cross_f64_blockdata_3x1() {
        let context = StubContext::default();
        let p = Parameters::new();
        let mut cross_block =
            CrossProductBlock::<(Matrix<3, 1, f64>, Matrix<3, 1, f64>)>::default();
        let input1 = BlockData::new(3, 1, &[1.0, 0.0, 0.0]);
        let input2 = BlockData::new(3, 1, &[0.0, 1.0, 0.0]);
        cross_block.process(&p, &context, (&input1.to_pass(), &input2.to_pass()));

        assert_eq!(
            cross_block.data,
            BlockData::from_matrix(&[&[0.0], &[0.0], &[1.0]])
        );
    }

    #[test]
    fn test_vector_cross_f64_1x3() {
        let context = StubContext::default();
        let p = Parameters::new();
        let mut cross_block =
            CrossProductBlock::<(Matrix<1, 3, f64>, Matrix<1, 3, f64>)>::default();
        let input1: Matrix<1, 3, f64> = Matrix {
            data: [[1.0], [0.0], [0.0]],
        };
        let input2: Matrix<1, 3, f64> = Matrix {
            data: [[0.0], [1.0], [0.0]],
        };
        let output = cross_block.process(&p, &context, (&input1, &input2));
        assert_eq!(output.data, [[0.0], [0.0], [1.0]]);
    }

    #[test]
    fn test_vector_cross_f64_3x1() {
        let context = StubContext::default();
        let p = Parameters::new();
        let mut cross_block =
            CrossProductBlock::<(Matrix<3, 1, f64>, Matrix<3, 1, f64>)>::default();
        let input1: Matrix<3, 1, f64> = Matrix {
            data: [[1.0, 0.0, 0.0]],
        };
        let input2: Matrix<3, 1, f64> = Matrix {
            data: [[0.0, 1.0, 0.0]],
        };
        let output = cross_block.process(&p, &context, (&input1, &input2));
        assert_eq!(output.data, [[0.0, 0.0, 1.0]]);
    }

    #[test]
    fn test_vector_cross_f32_1x3() {
        let context = StubContext::default();
        let p = Parameters::new();
        let mut cross_block =
            CrossProductBlock::<(Matrix<1, 3, f32>, Matrix<1, 3, f32>)>::default();
        let input1: Matrix<1, 3, f32> = Matrix {
            data: [[1.0], [0.0], [0.0]],
        };
        let input2: Matrix<1, 3, f32> = Matrix {
            data: [[0.0], [1.0], [0.0]],
        };
        let output = cross_block.process(&p, &context, (&input1, &input2));
        assert_eq!(output.data, [[0.0], [0.0], [1.0]]);
    }

    #[test]
    fn test_vector_cross_f32_3x1() {
        let context = StubContext::default();
        let p = Parameters::new();
        let mut cross_block =
            CrossProductBlock::<(Matrix<3, 1, f32>, Matrix<3, 1, f32>)>::default();
        let input1: Matrix<3, 1, f32> = Matrix {
            data: [[1.0, 0.0, 0.0]],
        };
        let input2: Matrix<3, 1, f32> = Matrix {
            data: [[0.0, 1.0, 0.0]],
        };
        let output = cross_block.process(&p, &context, (&input1, &input2));
        assert_eq!(output.data, [[0.0, 0.0, 1.0]]);
    }
}
