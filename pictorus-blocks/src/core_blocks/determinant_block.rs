use block_data::{BlockData as OldBlockData, FromPass};
use nalgebra::{ArrayStorage, Const, DimMin, SquareMatrix, ToTypenum};
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock, Scalar};

pub struct Parameters {}

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

/// A block that calculates the determinant of a square matrix.
pub struct DeterminantBlock<S: Scalar, T> {
    pub data: OldBlockData,
    buffer: S,
    phantom_type: core::marker::PhantomData<T>,
}

impl<S, T> Default for DeterminantBlock<S, T>
where
    S: Default + Scalar + Pass,
    T: Pass,
    OldBlockData: FromPass<S>,
{
    fn default() -> Self {
        Self {
            data: <OldBlockData as FromPass<S>>::from_pass(S::default().as_by()),
            buffer: S::default(),
            phantom_type: core::marker::PhantomData,
        }
    }
}

macro_rules! impl_determinant_block {
    ($type:ty) => {
        impl<const N: usize> ProcessBlock for DeterminantBlock<$type, Matrix<N, N, $type>>
        where
            Const<N>: ToTypenum + DimMin<Const<N>, Output = Const<N>>,
            OldBlockData: FromPass<$type>,
        {
            type Inputs = Matrix<N, N, $type>;
            type Output = $type;
            type Parameters = Parameters;

            fn process<'b>(
                &'b mut self,
                _parameters: &Self::Parameters,
                _context: &dyn pictorus_traits::Context,
                inputs: PassBy<'_, Self::Inputs>,
            ) -> PassBy<'b, Self::Output> {
                self.buffer =
                    SquareMatrix::from_array_storage(ArrayStorage(inputs.data)).determinant();
                self.data = OldBlockData::from_scalar(self.buffer.into());
                self.buffer
            }
        }

        impl ProcessBlock for DeterminantBlock<$type, $type>
        where
            OldBlockData: FromPass<$type>,
        {
            type Inputs = $type;
            type Output = $type;
            type Parameters = Parameters;

            fn process<'b>(
                &'b mut self,
                _parameters: &Self::Parameters,
                _context: &dyn pictorus_traits::Context,
                inputs: PassBy<'_, Self::Inputs>,
            ) -> PassBy<'b, Self::Output> {
                self.buffer = inputs;
                self.data = OldBlockData::from_scalar(self.buffer.into());
                self.buffer
            }
        }
    };
}

impl_determinant_block!(f32);
impl_determinant_block!(f64);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use block_data::BlockDataType;
    use paste::paste;

    macro_rules! impl_determinant_tests {
        ($type:ty) => {
            paste! {
                #[test]
                fn [<test_vector_determinant_ $type>]() {
                    let mut det_block = DeterminantBlock::<$type, Matrix<2, 2, $type>>::default();

                    let c = StubContext::default();
                    let p = Parameters::new();
                    let input = Matrix {
                        data: [[1.0, 2.0], [3.0, 4.0]]
                    };
                    let output = det_block.process(&p, &c, &input);

                    assert!(output == -2.0);
                    assert!(det_block.data.scalar() == -2.0);
                    assert!(det_block.data.get_type() == BlockDataType::Scalar);

                    let mut det_block_3x3 = DeterminantBlock::<$type, Matrix<3, 3, $type>>::default();
                    let input_3x3 = Matrix {
                        data: [[2.0, 0.0, 1.0], [3.0, 4.0, 6.0], [1.0, 5.0, 2.0]]
                    };
                    let output = det_block_3x3.process(&p, &c, &input_3x3);
                    assert!(output == -33.0);
                    assert!(det_block_3x3.data.scalar() == -33.0);
                    assert!(det_block_3x3.data.get_type() == BlockDataType::Scalar);
                }
            }
        }
    }

    impl_determinant_tests!(f32);
    impl_determinant_tests!(f64);
}
