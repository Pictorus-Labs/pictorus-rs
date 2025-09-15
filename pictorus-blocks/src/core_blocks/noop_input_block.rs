use pictorus_traits::{Context, InputBlock, Pass, PassBy, Scalar};

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
/// but don't have a fixed message type, for example the uORB input block, which
/// has an output size based on which message is selected. Output data is the
/// default() value for the type.
pub struct NoOpInputBlock<T> {
    _phantom: core::marker::PhantomData<T>,
    store: Option<T>,
}

impl<T> Default for NoOpInputBlock<T>
where
    T: Pass + Default,
{
    fn default() -> Self {
        Self {
            _phantom: core::marker::PhantomData,
            store: None,
        }
    }
}

impl InputBlock for NoOpInputBlock<f64> {
    type Parameters = NoOpInputBlockParameters;
    type Output = f64;

    fn input(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn Context,
    ) -> PassBy<'_, Self::Output> {
        0.0
    }
}

impl InputBlock for NoOpInputBlock<f32> {
    type Parameters = NoOpInputBlockParameters;
    type Output = f32;

    fn input(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn Context,
    ) -> PassBy<'_, Self::Output> {
        0.0
    }
}

impl<const NROWS: usize, const NCOLS: usize, T> InputBlock
    for NoOpInputBlock<pictorus_traits::Matrix<NROWS, NCOLS, T>>
where
    T: Scalar,
{
    type Parameters = NoOpInputBlockParameters;
    type Output = pictorus_traits::Matrix<NROWS, NCOLS, T>;

    fn input(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn Context,
    ) -> PassBy<'_, Self::Output> {
        self.store = Some(pictorus_traits::Matrix::<NROWS, NCOLS, T>::default());
        self.store.as_ref().expect("Store was not initialized").as_by()
    }
}

impl<A: Pass + Default, B: Pass + Default> InputBlock for NoOpInputBlock<(A, B)>
where
    (A, B): for<'a> Pass<By<'a> = (PassBy<'a, A>, PassBy<'a, B>)>,
{
    type Parameters = NoOpInputBlockParameters;
    type Output = (A, B);

    fn input(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn Context,
    ) -> PassBy<'_, Self::Output> {
        self.store = Some((A::default(), B::default()));
        self.store.as_ref().expect("Store was not initialized").as_by()
    }
}

impl<A: Pass + Default, B: Pass + Default, C: Pass + Default> InputBlock
    for NoOpInputBlock<(A, B, C)>
where
    (A, B, C): for<'a> Pass<By<'a> = (PassBy<'a, A>, PassBy<'a, B>, PassBy<'a, C>)>,
{
    type Parameters = NoOpInputBlockParameters;
    type Output = (A, B, C);

    fn input(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn Context,
    ) -> PassBy<'_, Self::Output> {
        self.store = Some((A::default(), B::default(), C::default()));
        self.store.as_ref().expect("Store was not initialized").as_by()
    }
}

impl<A: Pass + Default, B: Pass + Default, C: Pass + Default, D: Pass + Default> InputBlock
    for NoOpInputBlock<(A, B, C, D)>
where
    (A, B, C, D):
        for<'a> Pass<By<'a> = (PassBy<'a, A>, PassBy<'a, B>, PassBy<'a, C>, PassBy<'a, D>)>,
{
    type Parameters = NoOpInputBlockParameters;
    type Output = (A, B, C, D);

    fn input(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn Context,
    ) -> PassBy<'_, Self::Output> {
        self.store = Some((A::default(), B::default(), C::default(), D::default()));
        self.store.as_ref().expect("Store was not initialized").as_by()
    }
}

impl<
        A: Pass + Default,
        B: Pass + Default,
        C: Pass + Default,
        D: Pass + Default,
        E: Pass + Default,
    > InputBlock for NoOpInputBlock<(A, B, C, D, E)>
where
    (A, B, C, D, E): for<'a> Pass<
        By<'a> = (
            PassBy<'a, A>,
            PassBy<'a, B>,
            PassBy<'a, C>,
            PassBy<'a, D>,
            PassBy<'a, E>,
        ),
    >,
{
    type Parameters = NoOpInputBlockParameters;
    type Output = (A, B, C, D, E);

    fn input(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn Context,
    ) -> PassBy<'_, Self::Output> {
        self.store = Some((
            A::default(),
            B::default(),
            C::default(),
            D::default(),
            E::default(),
        ));
        self.store.as_ref().expect("Store was not initialized").as_by()
    }
}

impl<
        A: Pass + Default,
        B: Pass + Default,
        C: Pass + Default,
        D: Pass + Default,
        E: Pass + Default,
        F: Pass + Default,
    > InputBlock for NoOpInputBlock<(A, B, C, D, E, F)>
where
    (A, B, C, D, E, F): for<'a> Pass<
        By<'a> = (
            PassBy<'a, A>,
            PassBy<'a, B>,
            PassBy<'a, C>,
            PassBy<'a, D>,
            PassBy<'a, E>,
            PassBy<'a, F>,
        ),
    >,
{
    type Parameters = NoOpInputBlockParameters;
    type Output = (A, B, C, D, E, F);

    fn input(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn Context,
    ) -> PassBy<'_, Self::Output> {
        self.store = Some((
            A::default(),
            B::default(),
            C::default(),
            D::default(),
            E::default(),
            F::default(),
        ));
        self.store.as_ref().expect("Store was not initialized").as_by()
    }
}

impl<
        A: Pass + Default,
        B: Pass + Default,
        C: Pass + Default,
        D: Pass + Default,
        E: Pass + Default,
        F: Pass + Default,
        G: Pass + Default,
    > InputBlock for NoOpInputBlock<(A, B, C, D, E, F, G)>
where
    (A, B, C, D, E, F, G): for<'a> Pass<
        By<'a> = (
            PassBy<'a, A>,
            PassBy<'a, B>,
            PassBy<'a, C>,
            PassBy<'a, D>,
            PassBy<'a, E>,
            PassBy<'a, F>,
            PassBy<'a, G>,
        ),
    >,
{
    type Parameters = NoOpInputBlockParameters;
    type Output = (A, B, C, D, E, F, G);

    fn input(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn Context,
    ) -> PassBy<'_, Self::Output> {
        self.store = Some((
            A::default(),
            B::default(),
            C::default(),
            D::default(),
            E::default(),
            F::default(),
            G::default(),
        ));
        self.store.as_ref().expect("Store was not initialized").as_by()
    }
}

impl<
        A: Pass + Default,
        B: Pass + Default,
        C: Pass + Default,
        D: Pass + Default,
        E: Pass + Default,
        F: Pass + Default,
        G: Pass + Default,
        H: Pass + Default,
    > InputBlock for NoOpInputBlock<(A, B, C, D, E, F, G, H)>
where
    (A, B, C, D, E, F, G, H): for<'a> Pass<
        By<'a> = (
            PassBy<'a, A>,
            PassBy<'a, B>,
            PassBy<'a, C>,
            PassBy<'a, D>,
            PassBy<'a, E>,
            PassBy<'a, F>,
            PassBy<'a, G>,
            PassBy<'a, H>,
        ),
    >,
{
    type Parameters = NoOpInputBlockParameters;
    type Output = (A, B, C, D, E, F, G, H);

    fn input(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn Context,
    ) -> PassBy<'_, Self::Output> {
        self.store = Some((
            A::default(),
            B::default(),
            C::default(),
            D::default(),
            E::default(),
            F::default(),
            G::default(),
            H::default(),
        ));
        self.store.as_ref().expect("Store was not initialized").as_by()
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
