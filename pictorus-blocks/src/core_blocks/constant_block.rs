use pictorus_traits::{GeneratorBlock, Matrix, Pass, PassBy, Scalar};

pub struct Parameters<T> {
    pub constant: T,
}

impl<T> Parameters<T> {
    pub fn new(constant: T) -> Self {
        Self { constant }
    }
}

/// Outputs a constant numeric value.
pub struct ConstantBlock<T>
where
    T: Apply,
{
    buffer: T::Output,
}

impl<T> Default for ConstantBlock<T>
where
    T: Apply,
{
    fn default() -> Self {
        Self {
            buffer: <T::Output>::default(),
        }
    }
}

impl<T> GeneratorBlock for ConstantBlock<T>
where
    T: Apply,
{
    type Output = T::Output;
    type Parameters = Parameters<T>;

    fn generate(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
    ) -> pictorus_traits::PassBy<'_, Self::Output> {
        T::apply(&mut self.buffer, parameters)
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        self.buffer.as_by()
    }
}

pub trait Apply: Pass + Sized {
    type Output: Pass + Default;

    fn apply<'s>(
        store: &'s mut Self::Output,
        parameters: &Parameters<Self>,
    ) -> PassBy<'s, Self::Output>;
}

impl Apply for f64 {
    type Output = f64;

    fn apply<'s>(
        store: &'s mut Self::Output,
        parameters: &Parameters<Self>,
    ) -> PassBy<'s, Self::Output> {
        *store = parameters.constant;
        parameters.constant
    }
}

impl<const NROWS: usize, const NCOLS: usize, T> Apply for Matrix<NROWS, NCOLS, T>
where
    T: Scalar,
{
    type Output = Matrix<NROWS, NCOLS, T>;

    fn apply<'s>(
        store: &'s mut Self::Output,
        parameters: &Parameters<Self>,
    ) -> PassBy<'s, Self::Output> {
        *store = parameters.constant;
        store
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;

    #[test]
    fn test_constant_default_buffer_no_panic() {
        let block = ConstantBlock::<f64>::default();
        assert_eq!(block.buffer(), 0.0);

        let block = ConstantBlock::<Matrix<2, 2, f64>>::default();
        assert_eq!(block.buffer(), &Matrix::<2, 2, f64>::zeroed());
    }

    #[test]
    fn test_constant_scalar() {
        let mut block = ConstantBlock::<f64>::default();
        let parameters = Parameters::new(3.0);
        let context = StubContext::default();

        let output = block.generate(&parameters, &context);
        assert_eq!(output, 3.0);
        assert_eq!(block.buffer(), output);
    }

    #[test]
    fn test_constant_vector() {
        let mut block = ConstantBlock::<Matrix<1, 2, f64>>::default();
        let parameters = Parameters::new(Matrix {
            data: [[1.0], [2.0]],
        });
        let context = StubContext::default();

        let output = block.generate(&parameters, &context);
        assert_eq!(output.data[0][0], 1.0);
        assert_eq!(output.data[1][0], 2.0);

        assert_eq!(block.buffer().data[0][0], 1.0);
        assert_eq!(block.buffer().data[1][0], 2.0);
    }

    #[test]
    fn test_constant_matrix() {
        let mut block = ConstantBlock::<Matrix<2, 2, f64>>::default();
        let parameters = Parameters::new(Matrix {
            data: [[1.0, 3.0], [2.0, 4.0]],
        });
        let context = StubContext::default();

        let output = block.generate(&parameters, &context);
        assert_eq!(output.data[0][0], 1.0);
        assert_eq!(output.data[1][0], 2.0);
        assert_eq!(output.data[0][1], 3.0);
        assert_eq!(output.data[1][1], 4.0);
        assert_eq!(block.buffer().data[0][0], 1.0);
        assert_eq!(block.buffer().data[1][0], 2.0);
        assert_eq!(block.buffer().data[0][1], 3.0);
        assert_eq!(block.buffer().data[1][1], 4.0);
    }
}
