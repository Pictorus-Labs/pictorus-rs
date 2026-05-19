use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock, Promote, Promotion, Scalar};

/// Outputs the input data with an added bias (offset).
pub struct BiasBlock<B, T>
where
    B: Scalar,
    T: Apply<B>,
{
    buffer: T::Output,
}

impl<B, T> Default for BiasBlock<B, T>
where
    B: Scalar,
    T: Apply<B> + Default,
{
    fn default() -> Self {
        Self {
            buffer: <T::Output>::default(),
        }
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

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        self.buffer.as_by()
    }
}

pub trait Apply<B: Scalar>: Pass {
    type Output: Pass + Default;

    fn apply<'s>(
        store: &'s mut Self::Output,
        input: PassBy<Self>,
        offset: B,
    ) -> PassBy<'s, Self::Output>;
}

impl<B> Apply<B> for f64
where
    B: Promote<f64> + Scalar,
{
    type Output = Promotion<B, f64>;

    fn apply<'s>(
        store: &'s mut Self::Output,
        input: PassBy<Self>,
        offset: B,
    ) -> PassBy<'s, Self::Output> {
        let output =
            <B as Promote<f64>>::promote_left(offset) + <B as Promote<f64>>::promote_right(input);
        *store = output;
        output
    }
}

impl<B> Apply<B> for f32
where
    B: Promote<f32> + Scalar,
{
    type Output = Promotion<B, f32>;

    fn apply<'s>(
        store: &'s mut Self::Output,
        input: PassBy<Self>,
        offset: B,
    ) -> PassBy<'s, Self::Output> {
        let output =
            <B as Promote<f32>>::promote_left(offset) + <B as Promote<f32>>::promote_right(input);
        *store = output;
        output
    }
}

impl<const NROWS: usize, const NCOLS: usize, B, T> Apply<B> for Matrix<NROWS, NCOLS, T>
where
    T: Scalar,
    B: Promote<T>,
{
    type Output = Matrix<NROWS, NCOLS, Promotion<B, T>>;

    fn apply<'s>(
        store: &'s mut Self::Output,
        input: PassBy<Self>,
        offset: B,
    ) -> PassBy<'s, Self::Output> {
        for i in 0..NROWS {
            for j in 0..NCOLS {
                store.data[j][i] = <B as Promote<T>>::promote_left(offset)
                    + <B as Promote<T>>::promote_right(input.data[j][i]);
            }
        }
        store
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
    fn test_bias_default_buffer_no_panic() {
        let block = BiasBlock::<f64, f64>::default();
        assert_eq!(block.buffer(), 0.0);

        let block = BiasBlock::<f64, Matrix<2, 2, f64>>::default();
        assert_eq!(block.buffer(), &Matrix::<2, 2, f64>::zeroed());
    }

    #[test]
    fn test_bias_scalar() {
        let mut block = BiasBlock::<f64, f64>::default();
        let parameters = Parameters::new(3.0);
        let context = StubContext::default();

        let output = block.process(&parameters, &context, 2.0);
        assert_eq!(output, 5.0);
        assert_eq!(block.buffer(), output);
    }

    #[test]
    fn test_bias_scalar_to_pass() {
        let mut block = BiasBlock::<f64, f64>::default();
        let parameters = Parameters::new(3.0);
        let context = StubContext::default();

        let output = block.process(&parameters, &context, -3.1);
        assert_relative_eq!(output, -0.1);
        assert_relative_eq!(block.buffer(), -0.1);
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
        assert_eq!(block.buffer().data, [[3.0, 4.0], [5.0, 6.0]]);
    }

    #[test]
    fn test_bias_matrix_to_pass() {
        let mut block = BiasBlock::<f64, Matrix<2, 2, f64>>::default();
        let context = StubContext::default();
        let input = Matrix {
            data: [[1.0, 2.0], [3.0, 4.0]],
        };
        let parameters = Parameters::new(2.0);
        let output = block.process(&parameters, &context, &input);
        assert_eq!(output.data, [[3.0, 4.0], [5.0, 6.0]]);
        assert_eq!(block.buffer().data, [[3.0, 4.0], [5.0, 6.0]]);
    }
}
