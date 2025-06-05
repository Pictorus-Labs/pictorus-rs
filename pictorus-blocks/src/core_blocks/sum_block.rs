use crate::nalgebra_interop::MatrixExt;
use pictorus_block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock, Scalar};

/// Sums (adds or subtracts) all inputs together.
pub struct SumBlock<T: Summable>
where
    pictorus_block_data::BlockData: FromPass<<T as Summable>::Output>,
{
    store: Option<T::Output>,
    pub data: OldBlockData,
}

impl<T: Summable> Default for SumBlock<T>
where
    pictorus_block_data::BlockData: FromPass<<T as Summable>::Output>,
{
    fn default() -> Self {
        Self {
            store: None,
            data: <OldBlockData as FromPass<T::Output>>::from_pass(T::Output::default().as_by()),
        }
    }
}

impl<T> ProcessBlock for SumBlock<T>
where
    T: Summable,
    OldBlockData: FromPass<T::Output>,
{
    type Inputs = T;
    type Output = T::Output;
    type Parameters = T::Parameters;

    fn process(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        input: PassBy<Self::Inputs>,
    ) -> PassBy<Self::Output> {
        self.store = None;
        let result = T::get_sum(input, *parameters, &mut self.store);
        self.data = OldBlockData::from_pass(result);
        result
    }
}

trait SumScalar:
    Scalar
    + nalgebra::Scalar
    + core::ops::Neg<Output = Self>
    + core::ops::Add<Output = Self>
    + core::ops::Sub<Output = Self>
    + core::ops::AddAssign
    + core::ops::SubAssign
{
}
impl SumScalar for f32 {}
impl SumScalar for f64 {}

/// This trait is used to determine the output type of a sum operation
/// between two types, most importantly it can be used recursively. To get the output type for
/// a tuple of inputs. For an input of all scalars the output is scalar. For all inputs being a
/// single size of matrix, or a mix of scalars and a single size of matrix the output is a matrix
/// of that size.
pub trait TypePromotion<RHS> {
    type Output: Pass + Default;
}

/// A Scalar and a scalar outputs a scalar
impl<S: SumScalar> TypePromotion<S> for S {
    type Output = S;
}

/// A Scalar and a Matrix outputs a Matrix
impl<const R: usize, const C: usize, S: SumScalar> TypePromotion<S> for Matrix<R, C, S> {
    type Output = Matrix<R, C, S>;
}

/// A Matrix and a Scalar outputs a Matrix
impl<const R: usize, const C: usize, S: SumScalar> TypePromotion<Matrix<R, C, S>> for S {
    type Output = Matrix<R, C, S>;
}

/// A Matrix and a Matrix outputs a Matrix
impl<const R: usize, const C: usize, S: SumScalar> TypePromotion<Matrix<R, C, S>>
    for Matrix<R, C, S>
{
    type Output = Matrix<R, C, S>;
}

/// Recursive definition for 3 inputs
impl<A, B, C> TypePromotion<(B, C)> for A
where
    B: TypePromotion<C>,
    A: TypePromotion<<B as TypePromotion<C>>::Output>,
{
    type Output = <A as TypePromotion<B::Output>>::Output;
}

/// Recursive definition for 4 inputs
impl<A, B, C, D> TypePromotion<(B, C, D)> for A
where
    B: TypePromotion<(C, D)>,
    A: TypePromotion<B::Output>,
{
    type Output = <A as TypePromotion<B::Output>>::Output;
}

/// Recursive definition for 5 inputs
impl<A, B, C, D, E> TypePromotion<(B, C, D, E)> for A
where
    B: TypePromotion<(C, D, E)>,
    A: TypePromotion<B::Output>,
{
    type Output = <A as TypePromotion<B::Output>>::Output;
}

/// Recursive definition for 6 inputs
impl<A, B, C, D, E, F> TypePromotion<(B, C, D, E, F)> for A
where
    B: TypePromotion<(C, D, E, F)>,
    A: TypePromotion<B::Output>,
{
    type Output = <A as TypePromotion<B::Output>>::Output;
}

/// Recursive definition for 7 inputs
impl<A, B, C, D, E, F, G> TypePromotion<(B, C, D, E, F, G)> for A
where
    B: TypePromotion<(C, D, E, F, G)>,
    A: TypePromotion<B::Output>,
{
    type Output = <A as TypePromotion<B::Output>>::Output;
}

/// Recursive definition for 8 inputs
impl<A, B, C, D, E, F, G, H> TypePromotion<(B, C, D, E, F, G, H)> for A
where
    B: TypePromotion<(C, D, E, F, G, H)>,
    A: TypePromotion<B::Output>,
{
    type Output = <A as TypePromotion<B::Output>>::Output;
}

/// This trait allow the implementor to be "summed into" a destination type
/// A matrix can only be summed into a matrix of the same size, a scalar can be summed into
/// a matrix or another scalar
pub trait SumInto<DEST: Pass>: Pass {
    fn sum_into<'a>(
        input: PassBy<Self>,
        sum_type: SumType,
        dest: &'a mut Option<DEST>,
    ) -> PassBy<'a, DEST>;
}

/// Scalar summing into a scalar
impl<S: SumScalar> SumInto<S> for S {
    fn sum_into<'a>(
        input: PassBy<Self>,
        sum_type: SumType,
        dest: &'a mut Option<S>,
    ) -> PassBy<'a, S> {
        let dest = dest.get_or_insert(S::default());
        match sum_type {
            SumType::Addition => {
                *dest += input;
            }
            SumType::Subtraction => {
                *dest -= input;
            }
        }
        *dest
    }
}

/// Matrix summing into a matrix
impl<const R: usize, const C: usize, S: SumScalar> SumInto<Matrix<R, C, S>> for Matrix<R, C, S> {
    fn sum_into<'a>(
        input: PassBy<Self>,
        sum_type: SumType,
        dest: &'a mut Option<Matrix<R, C, S>>,
    ) -> PassBy<'a, Matrix<R, C, S>> {
        let dest = dest.get_or_insert(Matrix::<R, C, S>::zeroed());
        let orig_dest = dest.as_view().clone_owned();
        match sum_type {
            SumType::Addition => {
                orig_dest.add_to(&input.as_view(), &mut dest.as_view_mut());
            }
            SumType::Subtraction => {
                orig_dest.sub_to(&input.as_view(), &mut dest.as_view_mut());
            }
        }
        dest
    }
}

/// Scalar summing into a matrix
impl<const R: usize, const C: usize, S: SumScalar> SumInto<Matrix<R, C, S>> for S {
    fn sum_into<'a>(
        input: PassBy<Self>,
        sum_type: SumType,
        dest: &'a mut Option<Matrix<R, C, S>>,
    ) -> PassBy<'a, Matrix<R, C, S>> {
        let dest = dest.get_or_insert(Matrix::<R, C, S>::zeroed());
        let mut orig_dest = dest.as_view().clone_owned();
        match sum_type {
            SumType::Addition => {
                orig_dest = orig_dest.add_scalar(input);
            }
            SumType::Subtraction => {
                orig_dest = orig_dest.add_scalar(-input);
            }
        }
        dest.as_view_mut().copy_from(&orig_dest);
        dest
    }
}

/// This trait makes use of the two above , `SumInto` and `TypePromotion` to sum a tuple of inputs (or a single input)
pub trait Summable: Pass {
    type Output: Pass + Default;
    type Parameters: Copy;

    fn get_sum<'a>(
        input: PassBy<Self>,
        parameters: Self::Parameters,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output>;
}

/// Single scalar input
impl<S: SumScalar> Summable for S {
    type Output = S;
    type Parameters = Parameters<1>;

    fn get_sum<'a>(
        input: PassBy<Self>,
        parameters: Self::Parameters,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        Self::sum_into(input, parameters.operations[0], dest);
        dest.unwrap()
    }
}

/// Single matrix input
impl<const R: usize, const C: usize, S: SumScalar> Summable for Matrix<R, C, S> {
    type Output = Matrix<R, C, S>;
    type Parameters = Parameters<1>;

    fn get_sum<'a>(
        input: PassBy<Self>,
        parameters: Self::Parameters,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        Self::sum_into(input, parameters.operations[0], dest);
        dest.as_ref().unwrap()
    }
}

impl<A, B> Summable for (A, B)
where
    A: TypePromotion<B>,
    A: SumInto<A::Output>,
    B: SumInto<A::Output>,
{
    type Output = A::Output;
    type Parameters = Parameters<2>;

    fn get_sum<'a>(
        input: PassBy<Self>,
        parameters: Self::Parameters,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (a, b) = input;
        A::sum_into(a, parameters.operations[0], dest);
        B::sum_into(b, parameters.operations[1], dest)
    }
}

impl<A, B, C> Summable for (A, B, C)
where
    A: TypePromotion<(B, C)>,
    A: SumInto<A::Output>,
    B: SumInto<A::Output>,
    C: SumInto<A::Output>,
{
    type Output = A::Output;
    type Parameters = Parameters<3>;

    fn get_sum<'a>(
        input: PassBy<Self>,
        parameters: Self::Parameters,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (a, b, c) = input;
        A::sum_into(a, parameters.operations[0], dest);
        B::sum_into(b, parameters.operations[1], dest);
        C::sum_into(c, parameters.operations[2], dest)
    }
}

impl<A, B, C, D> Summable for (A, B, C, D)
where
    A: TypePromotion<(B, C, D)>,
    A: SumInto<A::Output>,
    B: SumInto<A::Output>,
    C: SumInto<A::Output>,
    D: SumInto<A::Output>,
{
    type Output = A::Output;
    type Parameters = Parameters<4>;

    fn get_sum<'a>(
        input: PassBy<Self>,
        parameters: Self::Parameters,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (a, b, c, d) = input;
        A::sum_into(a, parameters.operations[0], dest);
        B::sum_into(b, parameters.operations[1], dest);
        C::sum_into(c, parameters.operations[2], dest);
        D::sum_into(d, parameters.operations[3], dest)
    }
}

impl<A, B, C, D, E> Summable for (A, B, C, D, E)
where
    A: TypePromotion<(B, C, D, E)>,
    A: SumInto<A::Output>,
    B: SumInto<A::Output>,
    C: SumInto<A::Output>,
    D: SumInto<A::Output>,
    E: SumInto<A::Output>,
{
    type Output = A::Output;
    type Parameters = Parameters<5>;

    fn get_sum<'a>(
        input: PassBy<Self>,
        parameters: Self::Parameters,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (a, b, c, d, e) = input;
        A::sum_into(a, parameters.operations[0], dest);
        B::sum_into(b, parameters.operations[1], dest);
        C::sum_into(c, parameters.operations[2], dest);
        D::sum_into(d, parameters.operations[3], dest);
        E::sum_into(e, parameters.operations[4], dest)
    }
}

impl<A, B, C, D, E, F> Summable for (A, B, C, D, E, F)
where
    A: TypePromotion<(B, C, D, E, F)>,
    A: SumInto<A::Output>,
    B: SumInto<A::Output>,
    C: SumInto<A::Output>,
    D: SumInto<A::Output>,
    E: SumInto<A::Output>,
    F: SumInto<A::Output>,
{
    type Output = A::Output;
    type Parameters = Parameters<6>;

    fn get_sum<'a>(
        input: PassBy<Self>,
        parameters: Self::Parameters,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (a, b, c, d, e, f) = input;
        A::sum_into(a, parameters.operations[0], dest);
        B::sum_into(b, parameters.operations[1], dest);
        C::sum_into(c, parameters.operations[2], dest);
        D::sum_into(d, parameters.operations[3], dest);
        E::sum_into(e, parameters.operations[4], dest);
        F::sum_into(f, parameters.operations[5], dest)
    }
}

impl<A, B, C, D, E, F, G> Summable for (A, B, C, D, E, F, G)
where
    A: TypePromotion<(B, C, D, E, F, G)>,
    A: SumInto<A::Output>,
    B: SumInto<A::Output>,
    C: SumInto<A::Output>,
    D: SumInto<A::Output>,
    E: SumInto<A::Output>,
    F: SumInto<A::Output>,
    G: SumInto<A::Output>,
{
    type Output = A::Output;
    type Parameters = Parameters<7>;

    fn get_sum<'a>(
        input: PassBy<Self>,
        parameters: Self::Parameters,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (a, b, c, d, e, f, g) = input;
        A::sum_into(a, parameters.operations[0], dest);
        B::sum_into(b, parameters.operations[1], dest);
        C::sum_into(c, parameters.operations[2], dest);
        D::sum_into(d, parameters.operations[3], dest);
        E::sum_into(e, parameters.operations[4], dest);
        F::sum_into(f, parameters.operations[5], dest);
        G::sum_into(g, parameters.operations[6], dest)
    }
}

impl<A, B, C, D, E, F, G, H> Summable for (A, B, C, D, E, F, G, H)
where
    A: TypePromotion<(B, C, D, E, F, G, H)>,
    A: SumInto<A::Output>,
    B: SumInto<A::Output>,
    C: SumInto<A::Output>,
    D: SumInto<A::Output>,
    E: SumInto<A::Output>,
    F: SumInto<A::Output>,
    G: SumInto<A::Output>,
    H: SumInto<A::Output>,
{
    type Output = A::Output;
    type Parameters = Parameters<8>;

    fn get_sum<'a>(
        input: PassBy<Self>,
        parameters: Self::Parameters,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (a, b, c, d, e, f, g, h) = input;
        A::sum_into(a, parameters.operations[0], dest);
        B::sum_into(b, parameters.operations[1], dest);
        C::sum_into(c, parameters.operations[2], dest);
        D::sum_into(d, parameters.operations[3], dest);
        E::sum_into(e, parameters.operations[4], dest);
        F::sum_into(f, parameters.operations[5], dest);
        G::sum_into(g, parameters.operations[6], dest);
        H::sum_into(h, parameters.operations[7], dest)
    }
}

/// The type of sum to perform
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SumType {
    Addition,
    Subtraction,
}

/// The parameters for the sum block
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parameters<const NUM_INPUTS: usize> {
    pub operations: [SumType; NUM_INPUTS],
}

impl<const NUM_INPUTS: usize> Parameters<NUM_INPUTS> {
    /// This new function accepts a fixed size arrays of f64 because that is what codegen hands it currently
    /// It should be revisited when we tackle codegen changes
    pub fn new(input: [f64; NUM_INPUTS]) -> Self {
        let mut operations = [SumType::Addition; NUM_INPUTS];
        for (i, &val) in input.iter().enumerate() {
            if val < 0.0 {
                operations[i] = SumType::Subtraction;
            }
        }
        Self { operations }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use approx::assert_relative_eq;

    #[test]
    fn test_one_scalar() {
        let mut block = SumBlock::<f64>::default();
        let input = 3.0;
        let stub_context = StubContext::default();
        let parameters = Parameters {
            operations: [SumType::Addition],
        };
        let result = block.process(&parameters, &stub_context, input);
        assert_relative_eq!(result, 3.0);
    }

    #[test]
    fn test_one_matrix() {
        let mut block = SumBlock::<Matrix<2, 2, f64>>::default();
        let input = Matrix {
            data: [[1.0, 2.0], [3.0, 4.0]],
        };
        let stub_context = StubContext::default();
        let parameters = Parameters {
            operations: [SumType::Addition],
        };
        let result = block.process(&parameters, &stub_context, &input);
        assert_relative_eq!(
            result.data.as_flattened(),
            [[1.0, 2.0], [3.0, 4.0]].as_flattened()
        );
    }

    #[test]
    fn test_multiple_scalars() {
        let stub_context = StubContext::default();

        // Two Inputs
        let mut two_block = SumBlock::<(f64, f64)>::default();
        let input = (3.0, 4.0);
        let parameters = Parameters {
            operations: [SumType::Addition, SumType::Addition],
        };
        let result = two_block.process(&parameters, &stub_context, input);
        assert_relative_eq!(result, 7.0);

        let parameters = Parameters {
            operations: [SumType::Addition, SumType::Subtraction],
        };
        let result = two_block.process(&parameters, &stub_context, input);
        assert_relative_eq!(result, -1.0);

        let parameters = Parameters {
            operations: [SumType::Subtraction, SumType::Addition],
        };
        let result = two_block.process(&parameters, &stub_context, input);
        assert_relative_eq!(result, 1.0);

        let parameters = Parameters {
            operations: [SumType::Subtraction, SumType::Subtraction],
        };
        let result = two_block.process(&parameters, &stub_context, input);
        assert_relative_eq!(result, -7.0);

        // Three Inputs
        let mut three_block = SumBlock::<(f64, f64, f64)>::default();
        let input = (3.0, 4.0, 5.0);
        let parameters = Parameters {
            operations: [SumType::Addition, SumType::Addition, SumType::Addition],
        };
        let result = three_block.process(&parameters, &stub_context, input);
        assert_relative_eq!(result, 12.0);

        let parameters = Parameters {
            operations: [SumType::Addition, SumType::Addition, SumType::Subtraction],
        };
        let result = three_block.process(&parameters, &stub_context, input);
        assert_relative_eq!(result, 2.0);

        // Four Inputs
        let mut four_block = SumBlock::<(f64, f64, f64, f64)>::default();
        let input = (3.0, 4.0, 5.0, 6.0);
        let parameters = Parameters {
            operations: [
                SumType::Addition,
                SumType::Addition,
                SumType::Addition,
                SumType::Addition,
            ],
        };
        let result = four_block.process(&parameters, &stub_context, input);
        assert_relative_eq!(result, 18.0);

        // Five Inputs
        let mut five_block = SumBlock::<(f64, f64, f64, f64, f64)>::default();
        let input = (3.0, 4.0, 5.0, 6.0, 7.0);
        let parameters = Parameters {
            operations: [
                SumType::Addition,
                SumType::Addition,
                SumType::Addition,
                SumType::Addition,
                SumType::Addition,
            ],
        };
        let result = five_block.process(&parameters, &stub_context, input);
        assert_relative_eq!(result, 25.0);

        // Six Inputs
        let mut six_block = SumBlock::<(f64, f64, f64, f64, f64, f64)>::default();
        let input = (3.0, 4.0, 5.0, 6.0, 7.0, 8.0);
        let parameters = Parameters {
            operations: [
                SumType::Addition,
                SumType::Addition,
                SumType::Addition,
                SumType::Addition,
                SumType::Addition,
                SumType::Addition,
            ],
        };
        let result = six_block.process(&parameters, &stub_context, input);
        assert_relative_eq!(result, 33.0);

        // Seven Inputs
        let mut seven_block = SumBlock::<(f64, f64, f64, f64, f64, f64, f64)>::default();
        let input = (3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
        let parameters = Parameters {
            operations: [
                SumType::Addition,
                SumType::Addition,
                SumType::Addition,
                SumType::Addition,
                SumType::Addition,
                SumType::Addition,
                SumType::Addition,
            ],
        };
        let result = seven_block.process(&parameters, &stub_context, input);
        assert_relative_eq!(result, 42.0);

        // Eight Inputs
        let mut eight_block = SumBlock::<(f64, f64, f64, f64, f64, f64, f64, f64)>::default();
        let input = (3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0);
        let parameters = Parameters {
            operations: [
                SumType::Addition,
                SumType::Addition,
                SumType::Addition,
                SumType::Addition,
                SumType::Addition,
                SumType::Addition,
                SumType::Addition,
                SumType::Addition,
            ],
        };
        let result = eight_block.process(&parameters, &stub_context, input);
        assert_relative_eq!(result, 52.0);
    }

    #[test]
    fn test_multiple_matrices() {
        let stub_context = StubContext::default();

        // Two Inputs
        let mut two_block = SumBlock::<(Matrix<2, 2, f64>, Matrix<2, 2, f64>)>::default();
        let input = (
            &Matrix {
                data: [[1.0, 2.0], [3.0, 4.0]],
            },
            &Matrix {
                data: [[5.0, 6.0], [7.0, 8.0]],
            },
        );
        let parameters = Parameters {
            operations: [SumType::Addition, SumType::Addition],
        };
        let result = two_block.process(&parameters, &stub_context, input);
        assert_relative_eq!(
            result.data.as_flattened(),
            [[6.0, 8.0], [10.0, 12.0]].as_flattened()
        );

        let parameters = Parameters {
            operations: [SumType::Addition, SumType::Subtraction],
        };
        let result = two_block.process(&parameters, &stub_context, input);
        assert_relative_eq!(
            result.data.as_flattened(),
            [[-4.0, -4.0], [-4.0, -4.0]].as_flattened()
        );

        let parameters = Parameters {
            operations: [SumType::Subtraction, SumType::Addition],
        };
        let result = two_block.process(&parameters, &stub_context, input);
        assert_relative_eq!(
            result.data.as_flattened(),
            [[4.0, 4.0], [4.0, 4.0]].as_flattened()
        );

        let parameters = Parameters {
            operations: [SumType::Subtraction, SumType::Subtraction],
        };
        let result = two_block.process(&parameters, &stub_context, input);
        assert_relative_eq!(
            result.data.as_flattened(),
            [[-6.0, -8.0], [-10.0, -12.0]].as_flattened()
        );

        // Three Inputs
        let mut three_block =
            SumBlock::<(Matrix<2, 2, f64>, Matrix<2, 2, f64>, Matrix<2, 2, f64>)>::default();
        let input = (
            &Matrix {
                data: [[1.0, 2.0], [3.0, 4.0]],
            },
            &Matrix {
                data: [[5.0, 6.0], [7.0, 8.0]],
            },
            &Matrix {
                data: [[9.0, 10.0], [11.0, 12.0]],
            },
        );
        let parameters = Parameters {
            operations: [SumType::Addition, SumType::Addition, SumType::Addition],
        };
        let result = three_block.process(&parameters, &stub_context, input);
        assert_relative_eq!(
            result.data.as_flattened(),
            [[15.0, 18.0], [21.0, 24.0]].as_flattened()
        );

        // Four Inputs
        let mut four_block = SumBlock::<(
            Matrix<2, 2, f64>,
            Matrix<2, 2, f64>,
            Matrix<2, 2, f64>,
            Matrix<2, 2, f64>,
        )>::default();
        let input = (
            &Matrix {
                data: [[1.0, 2.0], [3.0, 4.0]],
            },
            &Matrix {
                data: [[5.0, 6.0], [7.0, 8.0]],
            },
            &Matrix {
                data: [[9.0, 10.0], [11.0, 12.0]],
            },
            &Matrix {
                data: [[13.0, 14.0], [15.0, 16.0]],
            },
        );
        let parameters = Parameters {
            operations: [
                SumType::Addition,
                SumType::Addition,
                SumType::Addition,
                SumType::Addition,
            ],
        };
        let result = four_block.process(&parameters, &stub_context, input);
        assert_relative_eq!(
            result.data.as_flattened(),
            [[28.0, 32.0], [36.0, 40.0]].as_flattened()
        );
    }

    #[test]
    fn test_mixed_scalars_and_matrices() {
        let stub_context = StubContext::default();

        // Two Inputs
        let mut two_block = SumBlock::<(f64, Matrix<2, 2, f64>)>::default();
        let input = (
            3.0,
            &Matrix {
                data: [[1.0, 2.0], [3.0, 4.0]],
            },
        );
        let parameters = Parameters {
            operations: [SumType::Addition, SumType::Addition],
        };
        let result = two_block.process(&parameters, &stub_context, input);
        assert_relative_eq!(
            result.data.as_flattened(),
            [[4.0, 5.0], [6.0, 7.0]].as_flattened()
        );

        // Three Inputs
        let mut three_block_1 = SumBlock::<(f64, Matrix<2, 2, f64>, f64)>::default();
        let input = (
            3.0,
            &Matrix {
                data: [[1.0, 2.0], [3.0, 4.0]],
            },
            5.0,
        );
        let parameters = Parameters {
            operations: [SumType::Addition, SumType::Addition, SumType::Addition],
        };
        let result = three_block_1.process(&parameters, &stub_context, input);
        assert_relative_eq!(
            result.data.as_flattened(),
            [[9.0, 10.0], [11.0, 12.0]].as_flattened()
        );

        let mut three_block_2 = SumBlock::<(Matrix<2, 2, f64>, f64, Matrix<2, 2, f64>)>::default();
        let input = (
            &Matrix {
                data: [[1.0, 2.0], [3.0, 4.0]],
            },
            5.0,
            &Matrix {
                data: [[5.0, 6.0], [7.0, 8.0]],
            },
        );
        let parameters = Parameters {
            operations: [SumType::Addition, SumType::Addition, SumType::Addition],
        };
        let result = three_block_2.process(&parameters, &stub_context, input);
        assert_relative_eq!(
            result.data.as_flattened(),
            [[11.0, 13.0], [15.0, 17.0]].as_flattened()
        );
    }
}
