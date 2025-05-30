use pictorus_block_data::{BlockData as OldBlockData, FromPass};
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
    pub data: OldBlockData,
    buffer: Option<T::Output>,
}

impl<T> Default for ConstantBlock<T>
where
    T: Apply,
    OldBlockData: FromPass<T::Output>,
{
    fn default() -> Self {
        Self {
            buffer: None,
            data: <OldBlockData as FromPass<T::Output>>::from_pass(<T::Output>::default().as_by()),
        }
    }
}

impl<T> GeneratorBlock for ConstantBlock<T>
where
    T: Apply,
    OldBlockData: FromPass<T::Output>,
{
    type Output = T::Output;
    type Parameters = Parameters<T>;

    fn generate(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
    ) -> pictorus_traits::PassBy<Self::Output> {
        let output = T::apply(&mut self.buffer, parameters);
        self.data = OldBlockData::from_pass(output);
        output
    }
}

pub trait Apply: Pass + Sized {
    type Output: Pass + Default;

    fn apply<'s>(
        store: &'s mut Option<Self::Output>,
        parameters: &Parameters<Self>,
    ) -> PassBy<'s, Self::Output>;
}

impl Apply for f64 {
    type Output = f64;

    fn apply<'s>(
        store: &'s mut Option<Self::Output>,
        parameters: &Parameters<Self>,
    ) -> PassBy<'s, Self::Output> {
        *store = Some(parameters.constant);
        parameters.constant
    }
}

impl<const NROWS: usize, const NCOLS: usize, T> Apply for Matrix<NROWS, NCOLS, T>
where
    T: Scalar,
{
    type Output = Matrix<NROWS, NCOLS, T>;

    fn apply<'s>(
        store: &'s mut Option<Self::Output>,
        parameters: &Parameters<Self>,
    ) -> PassBy<'s, Self::Output> {
        let output = store.insert(Matrix::zeroed());
        *output = parameters.constant;
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use pictorus_block_data::{BlockData, ToPass};

    #[test]
    fn test_constant_scalar() {
        let mut block = ConstantBlock::<f64>::default();
        let parameters = Parameters::new(3.0);
        let context = StubContext::default();

        let output = block.generate(&parameters, &context);
        assert_eq!(output, 3.0);
        assert_eq!(block.data, BlockData::from_scalar(3.0));
    }

    #[test]
    fn test_constant_vector() {
        let vector: [f64; 2] = [1.0, 2.0];

        let mut block = ConstantBlock::<Matrix<1, 2, f64>>::default();
        let parameters = Parameters::new(BlockData::from_vector(&vector).to_pass());
        let context = StubContext::default();

        let output = block.generate(&parameters, &context); // <-- Converts Vector to Matrix in from_pass
        assert_eq!(output.data[0][0], 1.0);
        assert_eq!(output.data[1][0], 2.0);

        assert_eq!(block.data, BlockData::from_matrix(&[&vector]));
    }

    #[test]
    fn test_constant_matrix() {
        let matrix_as_blockdata = BlockData::from_matrix(&[&[1.0, 2.0], &[3.0, 4.0]]);

        let mut block = ConstantBlock::<Matrix<2, 2, f64>>::default();
        let parameters = Parameters::new(matrix_as_blockdata.to_pass());
        let context = StubContext::default();

        let output = block.generate(&parameters, &context);
        assert_eq!(output.data[0][0], 1.0);
        assert_eq!(output.data[1][0], 2.0);
        assert_eq!(output.data[0][1], 3.0);
        assert_eq!(output.data[1][1], 4.0);
        assert_eq!(block.data, matrix_as_blockdata);
    }
}
