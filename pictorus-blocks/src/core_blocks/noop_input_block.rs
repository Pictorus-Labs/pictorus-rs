use pictorus_traits::{Context, InputBlock, Pass, PassBy};

pub struct NoOpInputBlockParameters {}

impl NoOpInputBlockParameters {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for NoOpInputBlockParameters {
    fn default() -> Self {
        Self::new()
    }
}

/// This block is a special case for input blocks that need to be simulated
/// but don't have a pre-defined protocol, for example the uORB input block, which
/// has an output size based on which message is selected. Output data is the
/// default() value for the type.
pub struct NoOpInputBlock<T> {
    store: T,
}

impl<T> Default for NoOpInputBlock<T>
where
    T: Pass + Default,
{
    fn default() -> Self {
        Self {
            store: T::default(),
        }
    }
}

impl<T> InputBlock for NoOpInputBlock<T>
where
    T: Pass + Default,
{
    type Parameters = NoOpInputBlockParameters;
    type Output = T;

    fn input(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn Context,
    ) -> PassBy<'_, Self::Output> {
        self.store = T::default();
        self.store.as_by()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use pictorus_traits::Matrix;

    #[test]
    fn test_noop_input_block_scalars_and_scalar_tuples() {
        let mut block = NoOpInputBlock::<f64>::default();
        let params = NoOpInputBlockParameters::new();
        let context = StubContext::default();

        let input = block.input(&params, &context);
        assert_eq!(input, 0.0);

        let mut block = NoOpInputBlock::<f32>::default();
        let params = NoOpInputBlockParameters::new();
        let context = StubContext::default();
        let input = block.input(&params, &context);
        assert_eq!(input, 0.0);

        let mut block = NoOpInputBlock::<(f64, f64)>::default();
        let params = NoOpInputBlockParameters::new();
        let context = StubContext::default();
        let input = block.input(&params, &context);
        assert_eq!(input, (0.0, 0.0));

        let mut block = NoOpInputBlock::<(f32, f32, f32)>::default();
        let params = NoOpInputBlockParameters::new();
        let context = StubContext::default();
        let input = block.input(&params, &context);
        assert_eq!(input, (0.0, 0.0, 0.0));

        let mut block = NoOpInputBlock::<(f64, f64, f64, f64)>::default();
        let params = NoOpInputBlockParameters::new();
        let context = StubContext::default();
        let input = block.input(&params, &context);
        assert_eq!(input, (0.0, 0.0, 0.0, 0.0));

        let mut block = NoOpInputBlock::<(f32, f32, f32, f32, f32)>::default();
        let params = NoOpInputBlockParameters::new();
        let context = StubContext::default();
        let input = block.input(&params, &context);
        assert_eq!(input, (0.0, 0.0, 0.0, 0.0, 0.0));

        let mut block = NoOpInputBlock::<(f64, f64, f64, f64, f64, f64)>::default();
        let params = NoOpInputBlockParameters::new();
        let context = StubContext::default();
        let input = block.input(&params, &context);
        assert_eq!(input, (0.0, 0.0, 0.0, 0.0, 0.0, 0.0));

        let mut block = NoOpInputBlock::<(f32, f32, f32, f32, f32, f32, f32)>::default();
        let params = NoOpInputBlockParameters::new();
        let context = StubContext::default();
        let input = block.input(&params, &context);
        assert_eq!(input, (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0));

        let mut block = NoOpInputBlock::<(f64, f64, f64, f64, f64, f64, f64, f64)>::default();
        let params = NoOpInputBlockParameters::new();
        let context = StubContext::default();
        let input = block.input(&params, &context);
        assert_eq!(input, (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn test_noop_input_block_matrices() {
        let mut block = NoOpInputBlock::<Matrix<2, 2, f64>>::default();
        let params = NoOpInputBlockParameters::new();
        let context = StubContext::default();
        let input = block.input(&params, &context);
        assert_eq!(input, &Matrix::default());

        let mut block = NoOpInputBlock::<(Matrix<2, 2, f64>, Matrix<3, 1, f64>)>::default();
        let params = NoOpInputBlockParameters::new();
        let context = StubContext::default();
        let input = block.input(&params, &context);
        assert_eq!(input, (&Matrix::default(), &Matrix::default()));

        let mut block =
            NoOpInputBlock::<(Matrix<2, 2, f64>, Matrix<3, 1, f64>, Matrix<1, 3, f64>)>::default();
        let params = NoOpInputBlockParameters::new();
        let context = StubContext::default();
        let input = block.input(&params, &context);
        assert_eq!(
            input,
            (&Matrix::default(), &Matrix::default(), &Matrix::default())
        );

        let mut block = NoOpInputBlock::<(
            Matrix<2, 2, f64>,
            Matrix<3, 1, f64>,
            Matrix<1, 3, f64>,
            Matrix<2, 3, f64>,
        )>::default();
        let params = NoOpInputBlockParameters::new();
        let context = StubContext::default();
        let input = block.input(&params, &context);
        assert_eq!(
            input,
            (
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default()
            )
        );

        let mut block = NoOpInputBlock::<(
            Matrix<2, 2, f64>,
            Matrix<3, 1, f64>,
            Matrix<1, 3, f64>,
            Matrix<2, 3, f64>,
            Matrix<3, 2, f64>,
        )>::default();
        let params = NoOpInputBlockParameters::new();
        let context = StubContext::default();
        let input = block.input(&params, &context);
        assert_eq!(
            input,
            (
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default()
            )
        );

        let mut block = NoOpInputBlock::<(
            Matrix<2, 2, f64>,
            Matrix<3, 1, f64>,
            Matrix<1, 3, f64>,
            Matrix<2, 3, f64>,
            Matrix<3, 2, f64>,
            Matrix<1, 1, f64>,
        )>::default();
        let params = NoOpInputBlockParameters::new();
        let context = StubContext::default();
        let input = block.input(&params, &context);
        assert_eq!(
            input,
            (
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default()
            )
        );

        let mut block = NoOpInputBlock::<(
            Matrix<2, 2, f64>,
            Matrix<3, 1, f64>,
            Matrix<1, 3, f64>,
            Matrix<2, 3, f64>,
            Matrix<3, 2, f64>,
            Matrix<1, 1, f64>,
            Matrix<2, 1, f64>,
        )>::default();
        let params = NoOpInputBlockParameters::new();
        let context = StubContext::default();
        let input = block.input(&params, &context);
        assert_eq!(
            input,
            (
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default()
            )
        );

        let mut block = NoOpInputBlock::<(
            Matrix<2, 2, f64>,
            Matrix<3, 1, f64>,
            Matrix<1, 3, f64>,
            Matrix<2, 3, f64>,
            Matrix<3, 2, f64>,
            Matrix<1, 1, f64>,
            Matrix<2, 1, f64>,
            Matrix<1, 2, f64>,
        )>::default();
        let params = NoOpInputBlockParameters::new();
        let context = StubContext::default();
        let input = block.input(&params, &context);
        assert_eq!(
            input,
            (
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default(),
                &Matrix::default()
            )
        );
    }

    #[test]
    fn test_noop_input_block_mixed() {
        let mut block = NoOpInputBlock::<(f64, Matrix<2, 2, f64>)>::default();
        let params = NoOpInputBlockParameters::new();
        let context = StubContext::default();
        let input = block.input(&params, &context);
        assert_eq!(input, (0.0, &Matrix::default()));

        let mut block = NoOpInputBlock::<(f32, Matrix<3, 1, f32>, f32)>::default();
        let params = NoOpInputBlockParameters::new();
        let context = StubContext::default();
        let input = block.input(&params, &context);
        assert_eq!(input, (0.0, &Matrix::default(), 0.0));

        let mut block =
            NoOpInputBlock::<(f64, Matrix<2, 2, f64>, f64, Matrix<1, 3, f64>)>::default();
        let params = NoOpInputBlockParameters::new();
        let context = StubContext::default();
        let input = block.input(&params, &context);
        assert_eq!(input, (0.0, &Matrix::default(), 0.0, &Matrix::default()));

        let mut block =
            NoOpInputBlock::<(f64, Matrix<2, 2, f64>, f64, Matrix<3, 1, f64>, f64)>::default();
        let params = NoOpInputBlockParameters::new();
        let context = StubContext::default();
        let input = block.input(&params, &context);
        assert_eq!(
            input,
            (0.0, &Matrix::default(), 0.0, &Matrix::default(), 0.0)
        );

        let mut block = NoOpInputBlock::<(
            f64,
            Matrix<2, 2, f64>,
            f64,
            Matrix<3, 1, f64>,
            f64,
            Matrix<1, 3, f64>,
            f64,
        )>::default();
        let params = NoOpInputBlockParameters::new();
        let context = StubContext::default();
        let input = block.input(&params, &context);
        assert_eq!(
            input,
            (
                0.0,
                &Matrix::default(),
                0.0,
                &Matrix::default(),
                0.0,
                &Matrix::default(),
                0.0
            )
        );
    }
}
