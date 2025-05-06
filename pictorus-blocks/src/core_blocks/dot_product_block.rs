use core::ops::{AddAssign, Mul, MulAssign};

use crate::nalgebra_interop::MatrixExt;
use crate::traits::Scalar;
use block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock};

pub struct DotProductBlock<T: Apply> {
    buffer: Option<T::Output>,
    pub data: OldBlockData,
}

impl<T> Default for DotProductBlock<T>
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

impl<T> ProcessBlock for DotProductBlock<T>
where
    T: Apply,
    OldBlockData: FromPass<T::Output>,
{
    type Inputs = T;
    type Output = T::Output;
    type Parameters = Parameters;

    fn process<'b>(
        &'b mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
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

#[derive(Clone, Copy, Debug, Default)]
pub struct Parameters;

impl Parameters {
    pub fn new() -> Self {
        Parameters {}
    }
}

impl<const NROWS: usize, const NCOLS: usize, T> Apply
    for (Matrix<NROWS, NCOLS, T>, Matrix<NROWS, NCOLS, T>)
where
    T: Scalar + Default + num_traits::Zero + AddAssign<T> + MulAssign<T> + Mul<T, Output = T>,
{
    type Output = T;

    fn apply<'s>(
        store: &'s mut Option<Self::Output>,
        input: PassBy<Self>,
    ) -> PassBy<'s, Self::Output> {
        let (lhs, rhs) = input;
        let output = lhs.as_view().dot(&rhs.as_view());
        *store = Some(output);
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use crate::traits::MatrixOps;

    #[test]
    fn test_dot_product_block() {
        let mut block = DotProductBlock::<(Matrix<2, 1, f64>, Matrix<2, 1, f64>)>::default();
        let context = StubContext::default();
        let parameters = Parameters::new();
        let input = (&Matrix::from_element(2.0), &Matrix::from_element(6.0));
        let output = block.process(&parameters, &context, input);
        assert_eq!(output, 24.0);

        let mut block = DotProductBlock::<(Matrix<1, 2, u16>, Matrix<1, 2, u16>)>::default();
        let context = StubContext::default();
        let parameters = Parameters::new();
        let input = (&Matrix { data: [[3], [4]] }, &Matrix { data: [[6], [2]] });
        let output = block.process(&parameters, &context, input);
        assert_eq!(output, 26);
    }
}
