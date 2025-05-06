use crate::nalgebra_interop::MatrixExt;
use block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock};

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

/// Emits a norm (scalar magnitude) of the input vector. More specifically, it
/// computes the Frobenius norm of a matrix, which is a generalization of the
/// Euclidean norm for matrices.
pub struct VectorNormBlock<T>
where
    T: Apply,
{
    pub data: OldBlockData,
    buffer: Option<T::Output>,
}

impl<T> Default for VectorNormBlock<T>
where
    T: Apply,
    OldBlockData: FromPass<T::Output>,
{
    fn default() -> Self {
        Self {
            data: <OldBlockData as FromPass<T::Output>>::from_pass(T::Output::default().as_by()),
            buffer: None,
        }
    }
}

impl<T> ProcessBlock for VectorNormBlock<T>
where
    T: Apply,
    OldBlockData: FromPass<T::Output>,
{
    type Inputs = T;
    type Output = T::Output;
    type Parameters = Parameters;

    fn process(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<Self::Output> {
        let output = T::apply(&mut self.buffer, inputs);
        self.data = OldBlockData::from_pass(output);
        output
    }
}

pub trait Apply: Pass {
    type Output: Pass + Default;

    fn apply<'s>(
        store: &'s mut Option<Self::Output>,
        input: PassBy<Self>,
    ) -> PassBy<'s, Self::Output>;
}

// Promote i8, u8, i16, u16, i32, and u32
macro_rules! impl_vector_norm_apply {
    ($type:ty, $otype:ty) => {
        impl<const ROWS: usize, const COLS: usize> Apply for Matrix<ROWS, COLS, $type> {
            type Output = $otype;

            fn apply<'s>(
                store: &'s mut Option<Self::Output>,
                input: PassBy<Self>,
            ) -> PassBy<'s, Self::Output> {
                let mut output = Matrix::<ROWS, COLS, $otype>::zeroed();
                for r in 0..ROWS {
                    for c in 0..COLS {
                        output.data[c][r] = input.data[c][r].into();
                    }
                }
                let n = output.as_view().norm();
                *store = Some(n);
                n
            }
        }
    };
}

impl_vector_norm_apply!(i8, f32);
impl_vector_norm_apply!(u8, f32);
impl_vector_norm_apply!(i16, f32);
impl_vector_norm_apply!(u16, f32);
impl_vector_norm_apply!(i32, f64);
impl_vector_norm_apply!(u32, f64);

// f32 and f64 don't need to be promoted
macro_rules! impl_vector_norm {
    ($type:ty) => {
        impl<const ROWS: usize, const COLS: usize> Apply for Matrix<ROWS, COLS, $type> {
            type Output = $type;

            fn apply<'s>(
                store: &'s mut Option<Self::Output>,
                input: PassBy<Self>,
            ) -> PassBy<'s, Self::Output> {
                let n = input.as_view().norm();
                *store = Some(n);
                n
            }
        }
    };
}

impl_vector_norm!(f32);
impl_vector_norm!(f64);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use paste::paste;

    macro_rules! test_vector_norm {
        ($type:ty) => {
            paste! {
                #[test]
                fn [<test_vector_norm_ $type>]() {
                    let mut block = VectorNormBlock::<Matrix<1, 2, $type>>::default();
                    let p = Parameters::new();
                    let c = StubContext::default();

                    let input = Matrix {
                        data: [[[<3_$type>]], [[<4 $type>]]]
                    };

                    let output = block.process(&p, &c, &input);
                    assert_eq!(output, [<5 $type>].into());
                    assert_eq!(block.data, OldBlockData::from_scalar([<5 $type>].into()));
                }

                #[test]
                fn [<test_matrix_norm_ $type>]() {
                    let mut block = VectorNormBlock::<Matrix<2, 2, $type>>::default();
                    let p = Parameters::new();
                    let c = StubContext::default();

                    let input = Matrix {
                        data: [[[<3 $type>], [<3 $type>]], [[<3 $type>], [<3 $type>]]],
                    };
                    let output = block.process(&p, &c, &input);
                    assert_eq!(output, [<6 $type>].into());
                    assert_eq!(block.data, OldBlockData::from_scalar([<6 $type>].into()));
                }
            }
        };
    }

    test_vector_norm!(i8);
    test_vector_norm!(u8);
    test_vector_norm!(i16);
    test_vector_norm!(u16);
    test_vector_norm!(i32);
    test_vector_norm!(u32);
    test_vector_norm!(f32);
    test_vector_norm!(f64);
}
