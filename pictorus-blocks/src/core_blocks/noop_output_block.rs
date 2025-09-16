use pictorus_traits::{OutputBlock, Pass};

pub struct NoOpOutputBlockParameters {}

impl NoOpOutputBlockParameters {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for NoOpOutputBlockParameters {
    fn default() -> Self {
        Self::new()
    }
}

/// This block is a special case for output blocks that need to be simulated
/// but don't have a pre-defined protocol, for example the uORB output block, which
/// has an input size based on which message is selected. Input data is ignored
/// by this block.
pub struct NoOpOutputBlock<T> {
    _phantom: core::marker::PhantomData<T>,
}

impl<T> Default for NoOpOutputBlock<T> {
    fn default() -> Self {
        Self {
            _phantom: core::marker::PhantomData::<T>,
        }
    }
}

impl<T> OutputBlock for NoOpOutputBlock<T>
where
    T: Pass,
{
    type Parameters = NoOpOutputBlockParameters;
    type Inputs = T;

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        _inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) {
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use pictorus_traits::Matrix;

    #[test]
    fn test_noop_output_block_scalars() {
        let mut block = NoOpOutputBlock::<f64>::default();
        let params = NoOpOutputBlockParameters::new();
        let context = StubContext::default();

        block.output(&params, &context, 42.0);

        let mut block = NoOpOutputBlock::<f32>::default();
        let params = NoOpOutputBlockParameters::new();
        let context = StubContext::default();
        block.output(&params, &context, 42.0_f32);

        let mut block = NoOpOutputBlock::<(f64, f64)>::default();
        let params = NoOpOutputBlockParameters::new();
        let context = StubContext::default();
        block.output(&params, &context, (42.0, 43.0));

        let mut block = NoOpOutputBlock::<(f64, f64, f64)>::default();
        let params = NoOpOutputBlockParameters::new();
        let context = StubContext::default();
        block.output(&params, &context, (42.0, 43.0, 44.0));

        let mut block = NoOpOutputBlock::<(f64, f64, f64, f64)>::default();
        let params = NoOpOutputBlockParameters::new();
        let context = StubContext::default();
        block.output(&params, &context, (42.0, 43.0, 44.0, 45.0));

        let mut block = NoOpOutputBlock::<(f64, f64, f64, f64, f64)>::default();
        let params = NoOpOutputBlockParameters::new();
        let context = StubContext::default();
        block.output(&params, &context, (42.0, 43.0, 44.0, 45.0, 46.0));

        let mut block = NoOpOutputBlock::<(f64, f64, f64, f64, f64, f64)>::default();
        let params = NoOpOutputBlockParameters::new();
        let context = StubContext::default();
        block.output(&params, &context, (42.0, 43.0, 44.0, 45.0, 46.0, 47.0));

        let mut block = NoOpOutputBlock::<(f64, f64, f64, f64, f64, f64, f64)>::default();
        let params = NoOpOutputBlockParameters::new();
        let context = StubContext::default();
        block.output(
            &params,
            &context,
            (42.0, 43.0, 44.0, 45.0, 46.0, 47.0, 48.0),
        );

        let mut block = NoOpOutputBlock::<(f64, f64, f64, f64, f64, f64, f64, f64)>::default();
        let params = NoOpOutputBlockParameters::new();
        let context = StubContext::default();
        block.output(
            &params,
            &context,
            (42.0, 43.0, 44.0, 45.0, 46.0, 47.0, 48.0, 49.0),
        );
    }

    #[test]
    fn test_noop_output_block_matrix() {
        let mut block = NoOpOutputBlock::<Matrix<2, 2, f64>>::default();
        let params = NoOpOutputBlockParameters::new();
        let context = StubContext::default();
        block.output(&params, &context, &Matrix::default());

        let mut block = NoOpOutputBlock::<Matrix<3, 1, f32>>::default();
        let params = NoOpOutputBlockParameters::new();
        let context = StubContext::default();
        block.output(&params, &context, &Matrix::default());

        let mut block = NoOpOutputBlock::<(Matrix<1, 3, f64>, Matrix<2, 2, f64>)>::default();
        let params = NoOpOutputBlockParameters::new();
        let context = StubContext::default();
        block.output(&params, &context, (&Matrix::default(), &Matrix::default()));

        let mut block =
            NoOpOutputBlock::<(Matrix<1, 3, f64>, Matrix<2, 2, f64>, Matrix<3, 1, f64>)>::default();
        let params = NoOpOutputBlockParameters::new();
        let context = StubContext::default();
        block.output(
            &params,
            &context,
            (&Matrix::default(), &Matrix::default(), &Matrix::default()),
        );

        let mut block = NoOpOutputBlock::<(
            Matrix<1, 3, f64>,
            Matrix<2, 2, f64>,
            Matrix<3, 1, f64>,
            Matrix<2, 3, f64>,
        )>::default();
        let params = NoOpOutputBlockParameters::new();
        let context = StubContext::default();
        block.output(
            &params,
            &context,
            (
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
            ),
        );

        let mut block = NoOpOutputBlock::<(
            Matrix<2, 2, f64>,
            Matrix<3, 1, f64>,
            Matrix<1, 3, f64>,
            Matrix<2, 3, f64>,
            Matrix<3, 2, f64>,
        )>::default();
        let params = NoOpOutputBlockParameters::new();
        let context = StubContext::default();
        block.output(
            &params,
            &context,
            (
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
            ),
        );

        let mut block = NoOpOutputBlock::<(
            Matrix<1, 3, f64>,
            Matrix<2, 2, f64>,
            Matrix<3, 1, f64>,
            Matrix<2, 3, f64>,
            Matrix<3, 2, f64>,
        )>::default();
        let params = NoOpOutputBlockParameters::new();
        let context = StubContext::default();
        block.output(
            &params,
            &context,
            (
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
            ),
        );

        let mut block = NoOpOutputBlock::<(
            Matrix<1, 3, f64>,
            Matrix<2, 2, f64>,
            Matrix<3, 1, f64>,
            Matrix<2, 3, f64>,
            Matrix<3, 2, f64>,
            Matrix<1, 1, f64>,
        )>::default();
        let params = NoOpOutputBlockParameters::new();
        let context = StubContext::default();
        block.output(
            &params,
            &context,
            (
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
            ),
        );

        let mut block = NoOpOutputBlock::<(
            Matrix<1, 3, f64>,
            Matrix<2, 2, f64>,
            Matrix<3, 1, f64>,
            Matrix<2, 3, f64>,
            Matrix<3, 2, f64>,
            Matrix<1, 1, f64>,
            Matrix<4, 4, f64>,
        )>::default();
        let params = NoOpOutputBlockParameters::new();
        let context = StubContext::default();
        block.output(
            &params,
            &context,
            (
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
            ),
        );

        let mut block = NoOpOutputBlock::<(
            Matrix<1, 3, f64>,
            Matrix<2, 2, f64>,
            Matrix<3, 1, f64>,
            Matrix<2, 3, f64>,
            Matrix<3, 2, f64>,
            Matrix<1, 1, f64>,
            Matrix<4, 4, f64>,
            Matrix<5, 5, f64>,
        )>::default();
        let params = NoOpOutputBlockParameters::new();
        let context = StubContext::default();
        block.output(
            &params,
            &context,
            (
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
            ),
        );
    }

    #[test]
    fn test_noop_output_block_mixed() {
        let mut block = NoOpOutputBlock::<(f64, Matrix<2, 2, f64>, f64)>::default();
        let params = NoOpOutputBlockParameters::new();
        let context = StubContext::default();
        block.output(&params, &context, (42.0, &Matrix::default(), 43.0));

        let mut block = NoOpOutputBlock::<(f32, Matrix<2, 2, f32>, f32)>::default();
        let params = NoOpOutputBlockParameters::new();
        let context = StubContext::default();
        block.output(&params, &context, (42.0_f32, &Matrix::default(), 43.0_f32));
    }
}
