use crate::nalgebra_interop::MatrixExt;
/// Functionality for matrix multiplication mode of the ProductBlock.
use crate::traits::Scalar;
use pictorus_traits::{Matrix, Pass, PassBy};

/// There are no parameters when the block is set to MatrixMultiply
/// we can use an empty struct as the ProcessBlock::Parameters.
#[derive(Default, Clone, Copy, Debug)]
pub struct ParametersMatrixMult {}

impl ParametersMatrixMult {
    pub fn new() -> Self {
        Self {}
    }
}

/// This trait is used to handle sizing constraints for matrix multiplication, and to
/// determine the output type of the multiplication.
pub trait MatMulSizing<RHS: Pass>: Pass {
    type Output: for<'a> Pass<By<'a> = &'a Self::Output> + Default;

    fn mat_mul(lhs: PassBy<Self>, rhs: PassBy<RHS>) -> Self::Output;
}

impl<
        const OROWS: usize,
        const OCOLS: usize,
        const INNER: usize,
        S: Scalar
            + core::ops::MulAssign
            + core::ops::Mul
            + core::ops::AddAssign
            + core::ops::Add
            + num_traits::Zero
            + num_traits::One,
    > MatMulSizing<Matrix<INNER, OCOLS, S>> for Matrix<OROWS, INNER, S>
{
    type Output = Matrix<OROWS, OCOLS, S>;

    fn mat_mul(lhs: PassBy<Self>, rhs: PassBy<Matrix<INNER, OCOLS, S>>) -> Self::Output {
        <Self::Output as MatrixExt>::from_view(&(lhs.as_view() * rhs.as_view()).as_view())
    }
}

/// Recursive Def for 3
impl<A, B, C> MatMulSizing<(B, C)> for A
where
    (B, C): for<'a> Pass<By<'a> = (PassBy<'a, B>, PassBy<'a, C>)>,
    C: Pass,
    B: MatMulSizing<C>,
    A: MatMulSizing<B::Output>,
{
    type Output = <A as MatMulSizing<B::Output>>::Output;
    fn mat_mul(lhs: PassBy<Self>, rhs: PassBy<(B, C)>) -> Self::Output {
        let (rhs1, rhs2) = rhs;
        A::mat_mul(lhs, B::mat_mul(rhs1, rhs2).as_by())
    }
}

/// Recursive Def for 4
impl<A: Pass, B: Pass, C: Pass, D: Pass> MatMulSizing<(B, C, D)> for A
where
    (B, C, D): for<'a> Pass<By<'a> = (PassBy<'a, B>, PassBy<'a, C>, PassBy<'a, D>)>,
    (C, D): for<'a> Pass<By<'a> = (PassBy<'a, C>, PassBy<'a, D>)>,
    B: MatMulSizing<(C, D)>,
    A: MatMulSizing<B::Output>,
{
    type Output = <A as MatMulSizing<B::Output>>::Output;

    fn mat_mul(lhs: PassBy<Self>, rhs: PassBy<(B, C, D)>) -> Self::Output {
        let (rhs1, rhs2, rhs3) = rhs;
        A::mat_mul(lhs, B::mat_mul(rhs1, (rhs2, rhs3)).as_by())
    }
}

/// Recursive Def for 5
impl<A: Pass, B: Pass, C: Pass, D: Pass, E: Pass> MatMulSizing<(B, C, D, E)> for A
where
    (B, C, D, E):
        for<'a> Pass<By<'a> = (PassBy<'a, B>, PassBy<'a, C>, PassBy<'a, D>, PassBy<'a, E>)>,
    (C, D, E): for<'a> Pass<By<'a> = (PassBy<'a, C>, PassBy<'a, D>, PassBy<'a, E>)>,
    B: MatMulSizing<(C, D, E)>,
    A: MatMulSizing<B::Output>,
{
    type Output = <A as MatMulSizing<B::Output>>::Output;

    fn mat_mul(lhs: PassBy<Self>, rhs: PassBy<(B, C, D, E)>) -> Self::Output {
        let (rhs1, rhs2, rhs3, rhs4) = rhs;
        A::mat_mul(lhs, B::mat_mul(rhs1, (rhs2, rhs3, rhs4)).as_by())
    }
}

/// Recursive Def for 6
impl<A: Pass, B: Pass, C: Pass, D: Pass, E: Pass, F: Pass> MatMulSizing<(B, C, D, E, F)> for A
where
    (B, C, D, E, F): for<'a> Pass<
        By<'a> = (
            PassBy<'a, B>,
            PassBy<'a, C>,
            PassBy<'a, D>,
            PassBy<'a, E>,
            PassBy<'a, F>,
        ),
    >,
    (C, D, E, F):
        for<'a> Pass<By<'a> = (PassBy<'a, C>, PassBy<'a, D>, PassBy<'a, E>, PassBy<'a, F>)>,
    B: MatMulSizing<(C, D, E, F)>,
    A: MatMulSizing<B::Output>,
{
    type Output = <A as MatMulSizing<B::Output>>::Output;

    fn mat_mul(lhs: PassBy<Self>, rhs: PassBy<(B, C, D, E, F)>) -> Self::Output {
        let (rhs1, rhs2, rhs3, rhs4, rhs5) = rhs;
        A::mat_mul(lhs, B::mat_mul(rhs1, (rhs2, rhs3, rhs4, rhs5)).as_by())
    }
}

/// Recursive Def for 7
impl<A: Pass, B: Pass, C: Pass, D: Pass, E: Pass, F: Pass, G: Pass> MatMulSizing<(B, C, D, E, F, G)>
    for A
where
    (B, C, D, E, F, G): for<'a> Pass<
        By<'a> = (
            PassBy<'a, B>,
            PassBy<'a, C>,
            PassBy<'a, D>,
            PassBy<'a, E>,
            PassBy<'a, F>,
            PassBy<'a, G>,
        ),
    >,
    (C, D, E, F, G): for<'a> Pass<
        By<'a> = (
            PassBy<'a, C>,
            PassBy<'a, D>,
            PassBy<'a, E>,
            PassBy<'a, F>,
            PassBy<'a, G>,
        ),
    >,
    B: MatMulSizing<(C, D, E, F, G)>,
    A: MatMulSizing<B::Output>,
{
    type Output = <A as MatMulSizing<B::Output>>::Output;

    fn mat_mul(lhs: PassBy<Self>, rhs: PassBy<(B, C, D, E, F, G)>) -> Self::Output {
        let (rhs1, rhs2, rhs3, rhs4, rhs5, rhs6) = rhs;
        A::mat_mul(
            lhs,
            B::mat_mul(rhs1, (rhs2, rhs3, rhs4, rhs5, rhs6)).as_by(),
        )
    }
}

/// Recursive Def for 8
impl<A: Pass, B: Pass, C: Pass, D: Pass, E: Pass, F: Pass, G: Pass, H: Pass>
    MatMulSizing<(B, C, D, E, F, G, H)> for A
where
    (B, C, D, E, F, G, H): for<'a> Pass<
        By<'a> = (
            PassBy<'a, B>,
            PassBy<'a, C>,
            PassBy<'a, D>,
            PassBy<'a, E>,
            PassBy<'a, F>,
            PassBy<'a, G>,
            PassBy<'a, H>,
        ),
    >,
    (C, D, E, F, G, H): for<'a> Pass<
        By<'a> = (
            PassBy<'a, C>,
            PassBy<'a, D>,
            PassBy<'a, E>,
            PassBy<'a, F>,
            PassBy<'a, G>,
            PassBy<'a, H>,
        ),
    >,
    B: MatMulSizing<(C, D, E, F, G, H)>,
    A: MatMulSizing<B::Output>,
{
    type Output = <A as MatMulSizing<B::Output>>::Output;

    fn mat_mul(lhs: PassBy<Self>, rhs: PassBy<(B, C, D, E, F, G, H)>) -> Self::Output {
        let (rhs1, rhs2, rhs3, rhs4, rhs5, rhs6, rhs7) = rhs;
        A::mat_mul(
            lhs,
            B::mat_mul(rhs1, (rhs2, rhs3, rhs4, rhs5, rhs6, rhs7)).as_by(),
        )
    }
}

pub trait ApplyMatMul: Pass {
    type Output: Pass + Default;
    fn mat_mul<'a>(
        input: PassBy<Self>,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output>;
}

impl<A, B> ApplyMatMul for (A, B)
where
    (A, B): for<'a> Pass<By<'a> = (PassBy<'a, A>, PassBy<'a, B>)>,
    B: Pass,
    A: MatMulSizing<B>,
{
    type Output = A::Output;
    fn mat_mul<'a>(
        input: PassBy<Self>,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (lhs, rhs) = input;
        let output = A::mat_mul(lhs, rhs);
        dest.insert(output)
    }
}

impl<A: Pass, B: Pass, C: Pass> ApplyMatMul for (A, B, C)
where
    (A, B, C): for<'a> Pass<By<'a> = (PassBy<'a, A>, PassBy<'a, B>, PassBy<'a, C>)>,
    (B, C): for<'a> Pass<By<'a> = (PassBy<'a, B>, PassBy<'a, C>)>,
    A: MatMulSizing<(B, C)>,
{
    type Output = <A as MatMulSizing<(B, C)>>::Output;

    fn mat_mul<'a>(
        input: PassBy<Self>,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (lhs, rhs1, rhs2) = input;
        let output = A::mat_mul(lhs, (rhs1, rhs2));
        dest.insert(output)
    }
}

impl<A: Pass, B: Pass, C: Pass, D: Pass> ApplyMatMul for (A, B, C, D)
where
    (A, B, C, D):
        for<'a> Pass<By<'a> = (PassBy<'a, A>, PassBy<'a, B>, PassBy<'a, C>, PassBy<'a, D>)>,
    (B, C, D): for<'a> Pass<By<'a> = (PassBy<'a, B>, PassBy<'a, C>, PassBy<'a, D>)>,
    A: MatMulSizing<(B, C, D)>,
{
    type Output = <A as MatMulSizing<(B, C, D)>>::Output;

    fn mat_mul<'a>(
        input: PassBy<Self>,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (lhs, rhs1, rhs2, rhs3) = input;
        let output = A::mat_mul(lhs, (rhs1, rhs2, rhs3));
        dest.insert(output)
    }
}

impl<A: Pass, B: Pass, C: Pass, D: Pass, E: Pass> ApplyMatMul for (A, B, C, D, E)
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
    (B, C, D, E):
        for<'a> Pass<By<'a> = (PassBy<'a, B>, PassBy<'a, C>, PassBy<'a, D>, PassBy<'a, E>)>,
    A: MatMulSizing<(B, C, D, E)>,
{
    type Output = <A as MatMulSizing<(B, C, D, E)>>::Output;

    fn mat_mul<'a>(
        input: PassBy<Self>,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (lhs, rhs1, rhs2, rhs3, rhs4) = input;
        let output = A::mat_mul(lhs, (rhs1, rhs2, rhs3, rhs4));
        dest.insert(output)
    }
}

impl<A: Pass, B: Pass, C: Pass, D: Pass, E: Pass, F: Pass> ApplyMatMul for (A, B, C, D, E, F)
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
    (B, C, D, E, F): for<'a> Pass<
        By<'a> = (
            PassBy<'a, B>,
            PassBy<'a, C>,
            PassBy<'a, D>,
            PassBy<'a, E>,
            PassBy<'a, F>,
        ),
    >,
    A: MatMulSizing<(B, C, D, E, F)>,
{
    type Output = <A as MatMulSizing<(B, C, D, E, F)>>::Output;

    fn mat_mul<'a>(
        input: PassBy<Self>,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (lhs, rhs1, rhs2, rhs3, rhs4, rhs5) = input;
        let output = A::mat_mul(lhs, (rhs1, rhs2, rhs3, rhs4, rhs5));
        dest.insert(output)
    }
}

impl<A: Pass, B: Pass, C: Pass, D: Pass, E: Pass, F: Pass, G: Pass> ApplyMatMul
    for (A, B, C, D, E, F, G)
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
    (B, C, D, E, F, G): for<'a> Pass<
        By<'a> = (
            PassBy<'a, B>,
            PassBy<'a, C>,
            PassBy<'a, D>,
            PassBy<'a, E>,
            PassBy<'a, F>,
            PassBy<'a, G>,
        ),
    >,
    A: MatMulSizing<(B, C, D, E, F, G)>,
{
    type Output = <A as MatMulSizing<(B, C, D, E, F, G)>>::Output;

    fn mat_mul<'a>(
        input: PassBy<Self>,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (lhs, rhs1, rhs2, rhs3, rhs4, rhs5, rhs6) = input;
        let output = A::mat_mul(lhs, (rhs1, rhs2, rhs3, rhs4, rhs5, rhs6));
        dest.insert(output)
    }
}

impl<A: Pass, B: Pass, C: Pass, D: Pass, E: Pass, F: Pass, G: Pass, H: Pass> ApplyMatMul
    for (A, B, C, D, E, F, G, H)
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
    (B, C, D, E, F, G, H): for<'a> Pass<
        By<'a> = (
            PassBy<'a, B>,
            PassBy<'a, C>,
            PassBy<'a, D>,
            PassBy<'a, E>,
            PassBy<'a, F>,
            PassBy<'a, G>,
            PassBy<'a, H>,
        ),
    >,
    A: MatMulSizing<(B, C, D, E, F, G, H)>,
{
    type Output = <A as MatMulSizing<(B, C, D, E, F, G, H)>>::Output;
    fn mat_mul<'a>(
        input: PassBy<Self>,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (lhs, rhs1, rhs2, rhs3, rhs4, rhs5, rhs6, rhs7) = input;
        let output = A::mat_mul(lhs, (rhs1, rhs2, rhs3, rhs4, rhs5, rhs6, rhs7));
        dest.insert(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_mat_mul_8_inputs() {
        // By testing up to 8 inputs, we are testing all the recursive definitions
        // for the MatMulSizing trait. I used a quick bit of python/numpy to generate
        // the expected result for this test.

        let matrix_a = Matrix::<10, 8, f64> {
            data: [
                [1.0, 2.0, -4.0, 3.0, -1.0, 3.0, 1.0, -4.0, -4.0, -4.0],
                [-2.0, -1.0, 2.0, -5.0, -3.0, -4.0, 2.0, 0.0, 4.0, -1.0],
                [2.0, -2.0, 0.0, 4.0, 1.0, 4.0, -3.0, 0.0, -2.0, 2.0],
                [-1.0, 2.0, -4.0, -3.0, -1.0, 3.0, -5.0, 4.0, 2.0, 4.0],
                [1.0, 2.0, -1.0, 1.0, 3.0, 4.0, -2.0, -2.0, 1.0, 3.0],
                [4.0, -3.0, -5.0, -2.0, 1.0, -1.0, -4.0, 0.0, 3.0, 3.0],
                [-3.0, 0.0, 4.0, 3.0, -4.0, -4.0, 2.0, -4.0, 2.0, -5.0],
                [1.0, -1.0, 0.0, -3.0, -2.0, -2.0, -2.0, 4.0, -1.0, 3.0],
            ],
        };
        let matrix_b = Matrix::<8, 6, f64> {
            data: [
                [1.0, -3.0, -1.0, 3.0, 2.0, -5.0, 4.0, -3.0],
                [3.0, -5.0, 4.0, 2.0, -1.0, -3.0, 1.0, 1.0],
                [2.0, 2.0, 1.0, -4.0, -3.0, -1.0, 1.0, -5.0],
                [-5.0, -3.0, 4.0, -5.0, 2.0, -3.0, 3.0, -2.0],
                [2.0, -3.0, 3.0, 1.0, 0.0, -5.0, 4.0, -2.0],
                [2.0, -5.0, 1.0, 1.0, -3.0, -1.0, 4.0, -1.0],
            ],
        };
        let matrix_c = Matrix::<6, 9, f64> {
            data: [
                [1.0, -1.0, 3.0, -1.0, -5.0, -5.0],
                [1.0, 0.0, 3.0, -5.0, -5.0, 2.0],
                [-2.0, -2.0, -2.0, -3.0, 4.0, -5.0],
                [1.0, 4.0, 3.0, 4.0, -2.0, -5.0],
                [-3.0, 1.0, -3.0, 2.0, 1.0, -4.0],
                [0.0, 3.0, 1.0, 0.0, -4.0, -4.0],
                [-4.0, 1.0, 0.0, 2.0, -3.0, 0.0],
                [4.0, -5.0, 2.0, 3.0, -5.0, 1.0],
                [3.0, -5.0, 3.0, -2.0, -1.0, -1.0],
            ],
        };
        let matrix_d = Matrix::<9, 7, f64> {
            data: [
                [-5.0, 1.0, -1.0, 0.0, -3.0, 1.0, 1.0, -5.0, 3.0],
                [-5.0, -2.0, -2.0, -3.0, -2.0, -4.0, 4.0, -4.0, -3.0],
                [-3.0, 1.0, -4.0, -2.0, 1.0, 2.0, -3.0, -5.0, -3.0],
                [-4.0, 2.0, 0.0, -2.0, -2.0, -5.0, 1.0, -1.0, -3.0],
                [-1.0, -5.0, 0.0, -3.0, 3.0, 3.0, 4.0, -1.0, -2.0],
                [4.0, 0.0, -5.0, 4.0, -5.0, 3.0, 3.0, 1.0, 2.0],
                [0.0, 2.0, 3.0, -3.0, 2.0, -4.0, -2.0, 3.0, 0.0],
            ],
        };
        let matrix_e = Matrix::<7, 5, f64> {
            data: [
                [2.0, 2.0, -3.0, -4.0, -2.0, -2.0, -5.0],
                [-5.0, -2.0, 3.0, -4.0, -5.0, 2.0, -5.0],
                [2.0, 0.0, -3.0, 0.0, -2.0, 2.0, -3.0],
                [-2.0, 2.0, 3.0, -3.0, -5.0, 1.0, 0.0],
                [-5.0, -2.0, -4.0, 3.0, -1.0, -3.0, 1.0],
            ],
        };
        let matrix_f = Matrix::<5, 4, f64> {
            data: [
                [0.0, 0.0, -5.0, -2.0, -1.0],
                [0.0, 2.0, -5.0, -3.0, 0.0],
                [0.0, -4.0, -1.0, -5.0, -3.0],
                [-3.0, -1.0, -3.0, -5.0, 3.0],
            ],
        };
        let matrix_g = Matrix::<4, 3, f64> {
            data: [
                [-1.0, -1.0, -2.0, -5.0],
                [2.0, -3.0, -1.0, -3.0],
                [-5.0, -5.0, 1.0, -4.0],
            ],
        };
        let matrix_h = Matrix::<3, 2, f64> {
            data: [[3.0, 0.0, -3.0], [4.0, 4.0, 2.0]],
        };
        let expected_result = Matrix::<10, 2, f64> {
            data: [
                [
                    217500.0, -130383.0, -55437.0, -1148064.0, 442575.0, -90993.0, -206112.0,
                    547242.0, 639762.0, 1005921.0,
                ],
                [
                    -1649556.0, 454066.0, 2007406.0, 3155026.0, -24484.0, -983966.0, 6146700.0,
                    -5598820.0, -1069350.0, -7224572.0,
                ],
            ],
        };

        let mut result = None;
        let output = <(
            Matrix<10, 8, f64>,
            Matrix<8, 6, f64>,
            Matrix<6, 9, f64>,
            Matrix<9, 7, f64>,
            Matrix<7, 5, f64>,
            Matrix<5, 4, f64>,
            Matrix<4, 3, f64>,
            Matrix<3, 2, f64>,
        ) as ApplyMatMul>::mat_mul(
            (
                &matrix_a, &matrix_b, &matrix_c, &matrix_d, &matrix_e, &matrix_f, &matrix_g,
                &matrix_h,
            ),
            &mut result,
        );

        assert_eq!(output, &expected_result);
    }
}
