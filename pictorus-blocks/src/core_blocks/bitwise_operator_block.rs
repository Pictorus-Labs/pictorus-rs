use crate::traits::{CopyInto, Scalar, SizePromotion};
use pictorus_block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock};

/// Performs a bitwise operation on the input values.
///
/// Float values are transformed to integers using a `floor` operation before
/// the bitwise operation is applied, and then casted back to float before being returned.
/// This block accepts 2-8 inputs, they must all be the same size/type.
/// The operation is applied component wise to each element if the input is a matrix.
pub struct BitwiseOperatorBlock<T>
where
    T: Apply,
    OldBlockData: FromPass<<T as Apply>::Output>,
{
    store: Option<T::Output>,
    pub data: OldBlockData,
}

impl<T> Default for BitwiseOperatorBlock<T>
where
    T: Apply,
    OldBlockData: FromPass<<T as Apply>::Output>,
{
    fn default() -> Self {
        Self {
            store: None,
            data: <OldBlockData as FromPass<T::Output>>::from_pass(T::Output::default().as_by()),
        }
    }
}

impl<T> ProcessBlock for BitwiseOperatorBlock<T>
where
    T: Apply,
    OldBlockData: FromPass<<T as Apply>::Output>,
{
    type Inputs = T;
    type Output = T::Output;
    type Parameters = Operation;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        let result = T::apply(inputs, *parameters, &mut self.store);
        self.data = OldBlockData::from_pass(result);
        result
    }
}

#[derive(Debug, Clone, Copy, strum::EnumString)]
pub enum Operation {
    And,
    Or,
    Xor,
}

impl Operation {
    pub fn new(value: &str) -> Self {
        value.parse().expect("Failed to parse operation")
    }
}

/// This trait defines the equivalent of `AndAssign`, `OrAssign`, and `XorAssign` for any input type
/// accepted as an edge input to the BitwiseOperatorBlock
pub trait BitOperations<RHS: Pass>: Pass + Default {
    fn operate_assign(lhs: &mut Self, rhs: PassBy<RHS>, operation: Operation);
}

// Impl for simple types that implement `AndAssign`, `OrAssign`, and `XorAssign`
macro_rules! impl_bitwise_operator_simple {
    ($type:ty) => {
        impl BitOperations<$type> for $type {
            fn operate_assign(lhs: &mut Self, rhs: PassBy<$type>, operation: Operation) {
                match operation {
                    Operation::And => *lhs &= rhs,
                    Operation::Or => *lhs |= rhs,
                    Operation::Xor => *lhs ^= rhs,
                }
            }
        }
    };
}
impl_bitwise_operator_simple!(u8);
impl_bitwise_operator_simple!(i8);
impl_bitwise_operator_simple!(u16);
impl_bitwise_operator_simple!(i16);
impl_bitwise_operator_simple!(u32);
impl_bitwise_operator_simple!(i32);

// Impl for float types that require casting to integer before applying the operation
impl BitOperations<f32> for f32 {
    fn operate_assign(lhs: &mut Self, rhs: PassBy<Self>, operation: Operation) {
        match operation {
            Operation::And => *lhs = (*lhs as i32 & rhs as i32) as f32,
            Operation::Or => *lhs = (*lhs as i32 | rhs as i32) as f32,
            Operation::Xor => *lhs = (*lhs as i32 ^ rhs as i32) as f32,
        }
    }
}

impl BitOperations<f64> for f64 {
    fn operate_assign(lhs: &mut Self, rhs: PassBy<Self>, operation: Operation) {
        match operation {
            Operation::And => *lhs = (*lhs as i64 & rhs as i64) as f64,
            Operation::Or => *lhs = (*lhs as i64 | rhs as i64) as f64,
            Operation::Xor => *lhs = (*lhs as i64 ^ rhs as i64) as f64,
        }
    }
}

impl<S: BitOperations<S> + Scalar, const NROWS: usize, const NCOLS: usize>
    BitOperations<Matrix<NROWS, NCOLS, S>> for Matrix<NROWS, NCOLS, S>
{
    fn operate_assign(lhs: &mut Self, rhs: PassBy<Self>, operation: Operation) {
        for r in 0..NROWS {
            for c in 0..NCOLS {
                S::operate_assign(&mut lhs.data[c][r], rhs.data[c][r].as_by(), operation);
            }
        }
    }
}

impl<S: BitOperations<S> + Scalar, const NROWS: usize, const NCOLS: usize> BitOperations<S>
    for Matrix<NROWS, NCOLS, S>
{
    fn operate_assign(lhs: &mut Self, rhs: PassBy<S>, operation: Operation) {
        for r in 0..NROWS {
            for c in 0..NCOLS {
                S::operate_assign(&mut lhs.data[c][r], rhs, operation);
            }
        }
    }
}

/// This trait is what allows us to accept 2-8 inputs to the BitwiseOperatorBlock
/// and apply the operation to them.
pub trait Apply: Pass {
    type Output: Pass + Default;

    fn apply<'a>(
        input: PassBy<Self>,
        operation: Operation,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output>;
}

impl<A, B> Apply for (A, B)
where
    A: Pass + CopyInto<A::Output> + SizePromotion<B>,
    A::Output: BitOperations<B>,
    B: Pass,
{
    type Output = A::Output;

    fn apply<'a>(
        input: PassBy<Self>,
        operation: Operation,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let mut output = A::Output::default();
        A::copy_into(input.0, &mut output);
        A::Output::operate_assign(&mut output, input.1, operation);

        dest.insert(output).as_by()
    }
}

impl<A, B, C> Apply for (A, B, C)
where
    A: SizePromotion<(B, C)> + Pass + CopyInto<A::Output>,
    A::Output: BitOperations<B> + BitOperations<C>,
    B: Pass,
    C: Pass,
{
    type Output = A::Output;

    fn apply<'a>(
        input: PassBy<Self>,
        operation: Operation,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let mut output = A::Output::default();
        A::copy_into(input.0, &mut output);
        <A::Output as BitOperations<B>>::operate_assign(&mut output, input.1, operation);
        <A::Output as BitOperations<C>>::operate_assign(&mut output, input.2, operation);
        dest.insert(output).as_by()
    }
}

impl<A, B, C, D> Apply for (A, B, C, D)
where
    A: SizePromotion<(B, C, D)> + Pass + CopyInto<A::Output>,
    A::Output: BitOperations<B> + BitOperations<C> + BitOperations<D>,
    B: Pass,
    C: Pass,
    D: Pass,
{
    type Output = A::Output;

    fn apply<'a>(
        input: PassBy<Self>,
        operation: Operation,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let mut output = A::Output::default();
        A::copy_into(input.0, &mut output);
        <A::Output as BitOperations<B>>::operate_assign(&mut output, input.1, operation);
        <A::Output as BitOperations<C>>::operate_assign(&mut output, input.2, operation);
        <A::Output as BitOperations<D>>::operate_assign(&mut output, input.3, operation);
        dest.insert(output).as_by()
    }
}

impl<A, B, C, D, E> Apply for (A, B, C, D, E)
where
    A: SizePromotion<(B, C, D, E)> + Pass + CopyInto<A::Output>,
    A::Output: BitOperations<B> + BitOperations<C> + BitOperations<D> + BitOperations<E>,
    B: Pass,
    C: Pass,
    D: Pass,
    E: Pass,
{
    type Output = A::Output;

    fn apply<'a>(
        input: PassBy<Self>,
        operation: Operation,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let mut output = A::Output::default();
        A::copy_into(input.0, &mut output);
        <A::Output as BitOperations<B>>::operate_assign(&mut output, input.1, operation);
        <A::Output as BitOperations<C>>::operate_assign(&mut output, input.2, operation);
        <A::Output as BitOperations<D>>::operate_assign(&mut output, input.3, operation);
        <A::Output as BitOperations<E>>::operate_assign(&mut output, input.4, operation);
        dest.insert(output).as_by()
    }
}

impl<A, B, C, D, E, F> Apply for (A, B, C, D, E, F)
where
    A: SizePromotion<(B, C, D, E, F)> + Pass + CopyInto<A::Output>,
    A::Output: BitOperations<B>
        + BitOperations<C>
        + BitOperations<D>
        + BitOperations<E>
        + BitOperations<F>,
    B: Pass,
    C: Pass,
    D: Pass,
    E: Pass,
    F: Pass,
{
    type Output = A::Output;

    fn apply<'a>(
        input: PassBy<Self>,
        operation: Operation,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let mut output = A::Output::default();
        A::copy_into(input.0, &mut output);
        <A::Output as BitOperations<B>>::operate_assign(&mut output, input.1, operation);
        <A::Output as BitOperations<C>>::operate_assign(&mut output, input.2, operation);
        <A::Output as BitOperations<D>>::operate_assign(&mut output, input.3, operation);
        <A::Output as BitOperations<E>>::operate_assign(&mut output, input.4, operation);
        <A::Output as BitOperations<F>>::operate_assign(&mut output, input.5, operation);
        dest.insert(output).as_by()
    }
}

impl<A, B, C, D, E, F, G> Apply for (A, B, C, D, E, F, G)
where
    A: SizePromotion<(B, C, D, E, F, G)> + Pass + CopyInto<A::Output>,
    A::Output: BitOperations<B>
        + BitOperations<C>
        + BitOperations<D>
        + BitOperations<E>
        + BitOperations<F>
        + BitOperations<G>,
    B: Pass,
    C: Pass,
    D: Pass,
    E: Pass,
    F: Pass,
    G: Pass,
{
    type Output = A::Output;

    fn apply<'a>(
        input: PassBy<Self>,
        operation: Operation,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let mut output = A::Output::default();
        A::copy_into(input.0, &mut output);
        <A::Output as BitOperations<B>>::operate_assign(&mut output, input.1, operation);
        <A::Output as BitOperations<C>>::operate_assign(&mut output, input.2, operation);
        <A::Output as BitOperations<D>>::operate_assign(&mut output, input.3, operation);
        <A::Output as BitOperations<E>>::operate_assign(&mut output, input.4, operation);
        <A::Output as BitOperations<F>>::operate_assign(&mut output, input.5, operation);
        <A::Output as BitOperations<G>>::operate_assign(&mut output, input.6, operation);
        dest.insert(output).as_by()
    }
}

impl<A, B, C, D, E, F, G, H> Apply for (A, B, C, D, E, F, G, H)
where
    A: SizePromotion<(B, C, D, E, F, G, H)> + Pass + CopyInto<A::Output>,
    A::Output: BitOperations<B>
        + BitOperations<C>
        + BitOperations<D>
        + BitOperations<E>
        + BitOperations<F>
        + BitOperations<G>
        + BitOperations<H>,
    B: Pass,
    C: Pass,
    D: Pass,
    E: Pass,
    F: Pass,
    G: Pass,
    H: Pass,
{
    type Output = A::Output;

    fn apply<'a>(
        input: PassBy<Self>,
        operation: Operation,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let mut output = A::Output::default();
        A::copy_into(input.0, &mut output);
        <A::Output as BitOperations<B>>::operate_assign(&mut output, input.1, operation);
        <A::Output as BitOperations<C>>::operate_assign(&mut output, input.2, operation);
        <A::Output as BitOperations<D>>::operate_assign(&mut output, input.3, operation);
        <A::Output as BitOperations<E>>::operate_assign(&mut output, input.4, operation);
        <A::Output as BitOperations<F>>::operate_assign(&mut output, input.5, operation);
        <A::Output as BitOperations<G>>::operate_assign(&mut output, input.6, operation);
        <A::Output as BitOperations<H>>::operate_assign(&mut output, input.7, operation);
        dest.insert(output).as_by()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::testing::StubContext;
    use paste::paste;

    macro_rules! test_bitwise_operator {
        ($type:ty) => {
            paste! {
                #[test]
                fn [<test_and_scalar_ $type>]() {
                    let mut block = BitwiseOperatorBlock::<($type, $type, $type, $type, $type, $type, $type, $type)>::default();
                    let context = StubContext::default();
                    let params = Operation::And;
                    let output = block.process(&params, &context, ([<255 $type>], [<27 $type>], [<8 $type>], [<27 $type>], [<27 $type>], [<27 $type>], [<27 $type>], [<27 $type>]));
                    assert_eq!(output, [<8 $type>]);
                }

                #[test]
                fn [<test_or_scalar_ $type>]() {
                    let mut block = BitwiseOperatorBlock::<($type, $type, $type, $type)>::default();
                    let context = StubContext::default();
                    let params = Operation::Or;
                    let output = block.process(&params, &context, ([<0 $type>], [<8 $type>], [<1 $type>], [<1 $type>]));
                    assert_eq!(output, [<9 $type>]);
                }

                #[test]
                fn [<test_xor_scalar_ $type>]() {
                    let mut block = BitwiseOperatorBlock::<($type, $type)>::default();
                    let context = StubContext::default();
                    let params = Operation::Xor;
                    let output = block.process(&params, &context, ([<1 $type>], [<1 $type>]));
                    assert_eq!(output, [<0 $type>]);
                }

                #[test]
                fn [<test_and_matrix_ $type>]() {
                    let mut block = BitwiseOperatorBlock::<(Matrix<2, 2, $type>, Matrix<2, 2, $type>, Matrix<2, 2, $type>, Matrix<2, 2, $type>)>::default();
                    let context = StubContext::default();
                    let params = Operation::And;
                    let input = (
                        &Matrix {
                            data: [[[<255 $type>], [<255 $type>]], [[<255 $type>], [<255 $type>]]],
                        },
                        &Matrix {
                            data: [[[<27 $type>], [<8 $type>]], [[<27 $type>], [<27 $type>]]],
                        },
                        &Matrix {
                            data: [[[<27 $type>], [<27 $type>]], [[<2 $type>], [<27 $type>]]],
                        },
                        &Matrix {
                            data: [[[<27 $type>], [<27 $type>]], [[<27 $type>], [<16 $type>]]],
                        },
                    );
                    let output = block.process(&params, &context, input);
                    assert_eq!(
                        output.data,
                        [[[<27 $type>], [<8 $type>]], [[<2 $type>], [<16 $type>]]]
                    );
                }

                #[test]
                fn [<test_and_mixed_scalar_matrix_ $type>]() {
                    let mut block = BitwiseOperatorBlock::<(Matrix<2, 2, $type>, $type, Matrix<2, 2, $type>, Matrix<2, 2, $type>)>::default();
                    let context = StubContext::default();
                    let params = Operation::And;
                    let input = (
                        &Matrix {
                            data: [[[<27 $type>], [<8 $type>]], [[<27 $type>], [<27 $type>]]],
                        },
                        [<255 $type>],
                        &Matrix {
                            data: [[[<27 $type>], [<27 $type>]], [[<2 $type>], [<27 $type>]]],
                        },
                        &Matrix {
                            data: [[[<27 $type>], [<27 $type>]], [[<27 $type>], [<16 $type>]]],
                        },
                    );
                    let output = block.process(&params, &context, input);
                    assert_eq!(
                        output.data,
                        [[[<27 $type>], [<8 $type>]], [[<2 $type>], [<16 $type>]]]
                    );
                }
            }
        }
    }
    test_bitwise_operator!(u8);
    test_bitwise_operator!(i8);
    test_bitwise_operator!(u16);
    test_bitwise_operator!(i16);
    test_bitwise_operator!(u32);
    test_bitwise_operator!(i32);
    test_bitwise_operator!(f32);
    test_bitwise_operator!(f64);
}
