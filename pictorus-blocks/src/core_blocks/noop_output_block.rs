use pictorus_traits::{Matrix, OutputBlock, Pass, Scalar};

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
/// but don't have a fixed message type, for example the uORB output block, which
/// has an input size based on which message is selected. Input data is ignored.
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

impl<const NROWS: usize, const NCOLS: usize, T> OutputBlock
    for NoOpOutputBlock<Matrix<NROWS, NCOLS, T>>
where
    T: Scalar,
{
    type Parameters = NoOpOutputBlockParameters;
    type Inputs = pictorus_traits::Matrix<NROWS, NCOLS, T>;

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        _inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) {
    }
}

impl OutputBlock for NoOpOutputBlock<f64> {
    type Parameters = NoOpOutputBlockParameters;
    type Inputs = f64;

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        _inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) {
    }
}

impl OutputBlock for NoOpOutputBlock<f32> {
    type Parameters = NoOpOutputBlockParameters;
    type Inputs = f32;

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        _inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) {
    }
}

impl<A, B> OutputBlock for NoOpOutputBlock<(A, B)>
where
    A: Pass,
    B: Pass,
{
    type Parameters = NoOpOutputBlockParameters;
    type Inputs = (A, B);

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        _inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) {
    }
}

impl<A, B, C> OutputBlock for NoOpOutputBlock<(A, B, C)>
where
    A: Pass,
    B: Pass,
    C: Pass,
{
    type Parameters = NoOpOutputBlockParameters;
    type Inputs = (A, B, C);

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        _inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) {
    }
}

impl<A, B, C, D> OutputBlock for NoOpOutputBlock<(A, B, C, D)>
where
    A: Pass,
    B: Pass,
    C: Pass,
    D: Pass,
{
    type Parameters = NoOpOutputBlockParameters;
    type Inputs = (A, B, C, D);

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        _inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) {
    }
}

impl<A, B, C, D, E> OutputBlock for NoOpOutputBlock<(A, B, C, D, E)>
where
    A: Pass,
    B: Pass,
    C: Pass,
    D: Pass,
    E: Pass,
{
    type Parameters = NoOpOutputBlockParameters;
    type Inputs = (A, B, C, D, E);

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        _inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) {
    }
}

impl<A, B, C, D, E, F> OutputBlock for NoOpOutputBlock<(A, B, C, D, E, F)>
where
    A: Pass,
    B: Pass,
    C: Pass,
    D: Pass,
    E: Pass,
    F: Pass,
{
    type Parameters = NoOpOutputBlockParameters;
    type Inputs = (A, B, C, D, E, F);

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        _inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) {
    }
}

impl<A, B, C, D, E, F, G> OutputBlock for NoOpOutputBlock<(A, B, C, D, E, F, G)>
where
    A: Pass,
    B: Pass,
    C: Pass,
    D: Pass,
    E: Pass,
    F: Pass,
    G: Pass,
{
    type Parameters = NoOpOutputBlockParameters;
    type Inputs = (A, B, C, D, E, F, G);

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        _inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) {
    }
}

impl<A, B, C, D, E, F, G, H> OutputBlock for NoOpOutputBlock<(A, B, C, D, E, F, G, H)>
where
    A: Pass,
    B: Pass,
    C: Pass,
    D: Pass,
    E: Pass,
    F: Pass,
    G: Pass,
    H: Pass,
{
    type Parameters = NoOpOutputBlockParameters;
    type Inputs = (A, B, C, D, E, F, G, H);

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
