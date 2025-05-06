/// Functionality for componentwise mode of the ProductBlock.
use crate::traits::{ApplyInto, MatrixOps, Scalar, SizePromotion};
use pictorus_traits::{Matrix, Pass, PassBy};

// For the ComponentWise method the PArameters needs a multiply/divide parameter for each
/// input signal
pub struct ParametersComponentWise<const N: usize> {
    pub operations: [ProductOperation; N],
}

impl<const N: usize> ParametersComponentWise<N> {
    /// This new function accepts a fixed size arrays of f64 because that is what codgen hands it currently
    /// It should be revisited when we tackle codegen changes
    pub fn new(input: [f64; N]) -> Self {
        let mut operations = [ProductOperation::Multiply; N];
        for (i, val) in input.iter().enumerate() {
            if *val < 0.0 {
                operations[i] = ProductOperation::Divide;
            }
        }
        Self { operations }
    }
}

/// One of these is associated with each input signal
#[derive(Clone, Copy, Debug)]
pub enum ProductOperation {
    Multiply,
    Divide,
}

// Scalar into Scalar
impl<S: Scalar + core::ops::MulAssign + core::ops::DivAssign + num_traits::One>
    ApplyInto<S, ProductOperation> for S
{
    fn apply_into<'a>(
        input: PassBy<Self>,
        params: &ProductOperation,
        dest: &'a mut Option<S>,
    ) -> PassBy<'a, S> {
        let dest = dest.get_or_insert(S::one());
        match params {
            ProductOperation::Multiply => *dest *= input,
            ProductOperation::Divide => *dest /= input,
        }

        dest.as_by()
    }
}

// Matrix into Matrix
impl<S: Scalar, const R: usize, const C: usize> ApplyInto<Matrix<R, C, S>, ProductOperation>
    for Matrix<R, C, S>
where
    S: core::ops::MulAssign + core::ops::DivAssign + num_traits::One,
{
    fn apply_into<'a>(
        input: PassBy<Self>,
        params: &ProductOperation,
        dest: &'a mut Option<Matrix<R, C, S>>,
    ) -> PassBy<'a, Matrix<R, C, S>> {
        let dest = dest.get_or_insert(Matrix::from_element(S::one()));
        input.for_each(|val, col, row| match params {
            ProductOperation::Multiply => dest.data[col][row] *= val,
            ProductOperation::Divide => dest.data[col][row] /= val,
        });
        dest.as_by()
    }
}

// Scalar into Matrix
impl<S: Scalar, const R: usize, const C: usize> ApplyInto<Matrix<R, C, S>, ProductOperation> for S
where
    S: core::ops::MulAssign + core::ops::DivAssign + num_traits::One,
{
    fn apply_into<'a>(
        input: PassBy<Self>,
        params: &ProductOperation,
        dest: &'a mut Option<Matrix<R, C, S>>,
    ) -> PassBy<'a, Matrix<R, C, S>> {
        let dest = dest.get_or_insert(Matrix::from_element(S::one()));

        dest.data
            .as_flattened_mut()
            .iter_mut()
            .for_each(|val| match params {
                ProductOperation::Multiply => *val *= input,
                ProductOperation::Divide => *val /= input,
            });

        dest.as_by()
    }
}

/// This is a remix of the [`crate::traits::Apply`] trait. It has been extended to support per signal parameter specification
pub trait ApplyComponentWise: Pass {
    type Parameters;
    type Output: Pass + Default;
    fn apply<'a>(
        input: PassBy<Self>,
        params: &Self::Parameters,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output>;
}

impl<A, B> ApplyComponentWise for (A, B)
where
    A: SizePromotion<B>,
    A: ApplyInto<A::Output, ProductOperation>,
    B: ApplyInto<A::Output, ProductOperation>,
{
    type Parameters = ParametersComponentWise<2>;
    type Output = A::Output;

    fn apply<'a>(
        input: PassBy<Self>,
        params: &Self::Parameters,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (lhs, rhs) = input;
        A::apply_into(lhs, &params.operations[0], dest);
        B::apply_into(rhs, &params.operations[1], dest)
    }
}

impl<A, B, C> ApplyComponentWise for (A, B, C)
where
    A: SizePromotion<(B, C)>,
    A: ApplyInto<A::Output, ProductOperation>,
    B: ApplyInto<A::Output, ProductOperation>,
    C: ApplyInto<A::Output, ProductOperation>,
{
    type Parameters = ParametersComponentWise<3>;
    type Output = A::Output;

    fn apply<'a>(
        input: PassBy<Self>,
        params: &Self::Parameters,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (lhs, rhs1, rhs2) = input;
        A::apply_into(lhs, &params.operations[0], dest);
        B::apply_into(rhs1, &params.operations[1], dest);
        C::apply_into(rhs2, &params.operations[2], dest)
    }
}

impl<A, B, C, D> ApplyComponentWise for (A, B, C, D)
where
    A: SizePromotion<(B, C, D)>,
    A: ApplyInto<A::Output, ProductOperation>,
    B: ApplyInto<A::Output, ProductOperation>,
    C: ApplyInto<A::Output, ProductOperation>,
    D: ApplyInto<A::Output, ProductOperation>,
{
    type Parameters = ParametersComponentWise<4>;
    type Output = A::Output;

    fn apply<'a>(
        input: PassBy<Self>,
        params: &Self::Parameters,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (lhs, rhs1, rhs2, rhs3) = input;
        A::apply_into(lhs, &params.operations[0], dest);
        B::apply_into(rhs1, &params.operations[1], dest);
        C::apply_into(rhs2, &params.operations[2], dest);
        D::apply_into(rhs3, &params.operations[3], dest)
    }
}

impl<A, B, C, D, E> ApplyComponentWise for (A, B, C, D, E)
where
    A: SizePromotion<(B, C, D, E)>,
    A: ApplyInto<A::Output, ProductOperation>,
    B: ApplyInto<A::Output, ProductOperation>,
    C: ApplyInto<A::Output, ProductOperation>,
    D: ApplyInto<A::Output, ProductOperation>,
    E: ApplyInto<A::Output, ProductOperation>,
{
    type Parameters = ParametersComponentWise<5>;
    type Output = A::Output;

    fn apply<'a>(
        input: PassBy<Self>,
        params: &Self::Parameters,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (lhs, rhs1, rhs2, rhs3, rhs4) = input;
        A::apply_into(lhs, &params.operations[0], dest);
        B::apply_into(rhs1, &params.operations[1], dest);
        C::apply_into(rhs2, &params.operations[2], dest);
        D::apply_into(rhs3, &params.operations[3], dest);
        E::apply_into(rhs4, &params.operations[4], dest)
    }
}

impl<A, B, C, D, E, F> ApplyComponentWise for (A, B, C, D, E, F)
where
    A: SizePromotion<(B, C, D, E, F)>,
    A: ApplyInto<A::Output, ProductOperation>,
    B: ApplyInto<A::Output, ProductOperation>,
    C: ApplyInto<A::Output, ProductOperation>,
    D: ApplyInto<A::Output, ProductOperation>,
    E: ApplyInto<A::Output, ProductOperation>,
    F: ApplyInto<A::Output, ProductOperation>,
{
    type Parameters = ParametersComponentWise<6>;
    type Output = A::Output;

    fn apply<'a>(
        input: PassBy<Self>,
        params: &Self::Parameters,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (lhs, rhs1, rhs2, rhs3, rhs4, rhs5) = input;
        A::apply_into(lhs, &params.operations[0], dest);
        B::apply_into(rhs1, &params.operations[1], dest);
        C::apply_into(rhs2, &params.operations[2], dest);
        D::apply_into(rhs3, &params.operations[3], dest);
        E::apply_into(rhs4, &params.operations[4], dest);
        F::apply_into(rhs5, &params.operations[5], dest)
    }
}

impl<A, B, C, D, E, F, G> ApplyComponentWise for (A, B, C, D, E, F, G)
where
    A: SizePromotion<(B, C, D, E, F, G)>,
    A: ApplyInto<A::Output, ProductOperation>,
    B: ApplyInto<A::Output, ProductOperation>,
    C: ApplyInto<A::Output, ProductOperation>,
    D: ApplyInto<A::Output, ProductOperation>,
    E: ApplyInto<A::Output, ProductOperation>,
    F: ApplyInto<A::Output, ProductOperation>,
    G: ApplyInto<A::Output, ProductOperation>,
{
    type Parameters = ParametersComponentWise<7>;
    type Output = A::Output;

    fn apply<'a>(
        input: PassBy<Self>,
        params: &Self::Parameters,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (lhs, rhs1, rhs2, rhs3, rhs4, rhs5, rhs6) = input;
        A::apply_into(lhs, &params.operations[0], dest);
        B::apply_into(rhs1, &params.operations[1], dest);
        C::apply_into(rhs2, &params.operations[2], dest);
        D::apply_into(rhs3, &params.operations[3], dest);
        E::apply_into(rhs4, &params.operations[4], dest);
        F::apply_into(rhs5, &params.operations[5], dest);
        G::apply_into(rhs6, &params.operations[6], dest)
    }
}

impl<A, B, C, D, E, F, G, H> ApplyComponentWise for (A, B, C, D, E, F, G, H)
where
    A: SizePromotion<(B, C, D, E, F, G, H)>,
    A: ApplyInto<A::Output, ProductOperation>,
    B: ApplyInto<A::Output, ProductOperation>,
    C: ApplyInto<A::Output, ProductOperation>,
    D: ApplyInto<A::Output, ProductOperation>,
    E: ApplyInto<A::Output, ProductOperation>,
    F: ApplyInto<A::Output, ProductOperation>,
    G: ApplyInto<A::Output, ProductOperation>,
    H: ApplyInto<A::Output, ProductOperation>,
{
    type Parameters = ParametersComponentWise<8>;
    type Output = A::Output;

    fn apply<'a>(
        input: PassBy<Self>,
        params: &Self::Parameters,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (lhs, rhs1, rhs2, rhs3, rhs4, rhs5, rhs6, rhs7) = input;
        A::apply_into(lhs, &params.operations[0], dest);
        B::apply_into(rhs1, &params.operations[1], dest);
        C::apply_into(rhs2, &params.operations[2], dest);
        D::apply_into(rhs3, &params.operations[3], dest);
        E::apply_into(rhs4, &params.operations[4], dest);
        F::apply_into(rhs5, &params.operations[5], dest);
        G::apply_into(rhs6, &params.operations[6], dest);
        H::apply_into(rhs7, &params.operations[7], dest)
    }
}
