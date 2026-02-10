use core::ops::Add;

use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock, Scalar};

/// Outputs the input data with an added bias (offset).
pub struct BiasBlock<B, T>
where
    B: Scalar,
    T: Apply<B>,
{
    buffer: Option<T::Output>,
}

impl<B, T> Default for BiasBlock<B, T>
where
    B: Scalar,
    T: Apply<B> + Default,
{
    fn default() -> Self {
        Self { buffer: None }
    }
}

impl<B, T> ProcessBlock for BiasBlock<B, T>
where
    B: Scalar,
    T: Apply<B> + Default,
{
    type Inputs = T;
    type Output = T::Output;
    type Parameters = Parameters<B>;

    fn process(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        input: PassBy<Self::Inputs>,
    ) -> PassBy<'_, Self::Output> {
        let output = T::apply(&mut self.buffer, input, parameters.offset);
        output
    }
}

pub trait Apply<B: Scalar>: Pass {
    type Output: Pass + Default;

    fn apply<'s>(
        store: &'s mut Option<Self::Output>,
        input: PassBy<Self>,
        offset: B,
    ) -> PassBy<'s, Self::Output>;
}

impl<B> Apply<B> for f64
where
    B: Scalar,
    f64: Add<B>,
    <f64 as Add<B>>::Output: Scalar,
{
    type Output = <f64 as Add<B>>::Output;

    fn apply<'s>(
        store: &'s mut Option<Self::Output>,
        input: PassBy<Self>,
        offset: B,
    ) -> PassBy<'s, Self::Output> {
        let output = <f64 as Add<B>>::add(input, offset);
        *store = Some(output);
        output
    }
}

impl<B> Apply<B> for f32
where
    B: Scalar,
    f32: Add<B>,
    <f32 as Add<B>>::Output: Scalar,
{
    type Output = <f32 as Add<B>>::Output;

    fn apply<'s>(
        store: &'s mut Option<Self::Output>,
        input: PassBy<Self>,
        offset: B,
    ) -> PassBy<'s, Self::Output> {
        let output = <f32 as Add<B>>::add(input, offset);
        *store = Some(output);
        output.as_by()
    }
}

impl<const NROWS: usize, const NCOLS: usize, B, T> Apply<B> for Matrix<NROWS, NCOLS, T>
where
    T: Scalar,
    B: Scalar,
    T: Add<B>,
    <T as Add<B>>::Output: Scalar,
{
    type Output = Matrix<NROWS, NCOLS, <T as Add<B>>::Output>;

    fn apply<'s>(
        store: &'s mut Option<Self::Output>,
        input: PassBy<Self>,
        offset: B,
    ) -> PassBy<'s, Self::Output> {
        let output = store.insert(Matrix::zeroed());
        for i in 0..NROWS {
            for j in 0..NCOLS {
                output.data[j][i] = <T as Add<B>>::add(input.data[j][i], offset);
            }
        }
        output
    }
}

pub struct Parameters<B>
where
    B: Scalar,
{
    pub offset: B,
}

impl<B> Parameters<B>
where
    B: Scalar,
{
    pub fn new(offset: B) -> Self {
        Self { offset }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use approx::assert_relative_eq;

    #[test]
    fn test_bias_scalar() {
        let mut block = BiasBlock::<f64, f64>::default();
        let parameters = Parameters::new(3.0);
        let context = StubContext::default();

        let output = block.process(&parameters, &context, 2.0);
        assert_eq!(output, 5.0);
    }

    #[test]
    fn test_bias_scalar_to_pass() {
        let mut block = BiasBlock::<f64, f64>::default();
        let parameters = Parameters::new(3.0);
        let context = StubContext::default();
        let input = -3.1;

        let output = block.process(&parameters, &context, input);
        assert_relative_eq!(output, -0.1);
    }

    #[test]
    fn test_bias_matrix() {
        let mut block = BiasBlock::<f64, Matrix<2, 2, f64>>::default();
        let context = StubContext::default();
        let input = Matrix {
            data: [[1.0, 2.0], [3.0, 4.0]],
        };
        let parameters = Parameters::new(2.0);
        let output = block.process(&parameters, &context, &input);
        assert_eq!(output.data, [[3.0, 4.0], [5.0, 6.0]]);
    }
}
