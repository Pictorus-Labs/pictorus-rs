use crate::traits::Scalar;
use core::marker::PhantomData;
use num_traits::{AsPrimitive, Float};
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock};

pub struct Parameters {}

impl Default for Parameters {
    fn default() -> Self {
        Self::new()
    }
}

impl Parameters {
    pub fn new() -> Self {
        Self {}
    }
}

/// Emits a norm (scalar magnitude) of the input vector.
///
/// More specifically, it computes the Frobenius norm of a matrix, which is a generalization of the
/// Euclidean norm for matrices.
///
/// The input can be a matrix of any scalar type; the output is a selectable float type
/// (`f64` by default) in which the sum of squares is accumulated.
pub struct VectorNormBlock<T, O: Scalar = f64>
where
    T: Apply<O>,
{
    buffer: O,
    phantom: PhantomData<T>,
}

impl<T, O: Scalar> Default for VectorNormBlock<T, O>
where
    T: Apply<O>,
{
    fn default() -> Self {
        Self {
            buffer: O::default(),
            phantom: PhantomData,
        }
    }
}

impl<T, O: Scalar> ProcessBlock for VectorNormBlock<T, O>
where
    T: Apply<O>,
{
    type Inputs = T;
    type Output = O;
    type Parameters = Parameters;

    fn process(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'_, Self::Output> {
        T::apply(&mut self.buffer, inputs)
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        self.buffer.as_by()
    }
}

pub trait Apply<O: Scalar>: Pass {
    fn apply<'s>(store: &'s mut O, input: PassBy<Self>) -> PassBy<'s, O>;
}

impl<const ROWS: usize, const COLS: usize, T, O> Apply<O> for Matrix<ROWS, COLS, T>
where
    T: Scalar + AsPrimitive<O>,
    O: Scalar + Float,
{
    fn apply<'s>(store: &'s mut O, input: PassBy<Self>) -> PassBy<'s, O> {
        let sum_of_squares = input
            .data
            .as_flattened()
            .iter()
            .fold(O::zero(), |acc, &v| {
                let val: O = v.as_();
                acc + val * val
            });
        let n = sum_of_squares.sqrt();
        *store = n;
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use paste::paste;

    #[test]
    fn test_vector_norm_default_buffer_no_panic() {
        let block = VectorNormBlock::<Matrix<1, 2, f64>>::default();
        assert_eq!(block.buffer(), 0.0);
    }

    #[test]
    fn test_vector_norm_default_output_type() {
        // Output type param defaults to f64 for any input type
        let mut block = VectorNormBlock::<Matrix<1, 2, i8>>::default();
        let p = Parameters::new();
        let c = StubContext::default();

        let input = Matrix { data: [[3], [4]] };
        let output: f64 = block.process(&p, &c, &input);
        assert_eq!(output, 5.0);
    }

    macro_rules! test_vector_norm {
        // Convenience call to generate a call to the main macro for every pair in the list
        ($type:ty : $otype:ty, $($rest_t:ty : $rest_o:ty),*) => {
            test_vector_norm!($type : $otype);
            test_vector_norm!($($rest_t : $rest_o),*);
        };
        ($type:ty : $otype:ty) => {
            paste! {
                #[test]
                fn [<test_vector_norm_ $type _to_ $otype>]() {
                    let mut block = VectorNormBlock::<Matrix<1, 2, $type>, $otype>::default();
                    let p = Parameters::new();
                    let c = StubContext::default();

                    let input = Matrix {
                        data: [[3 as $type], [4 as $type]]
                    };

                    let output = block.process(&p, &c, &input);
                    assert_eq!(output, 5 as $otype);
                    assert_eq!(block.buffer(), output);
                }

                #[test]
                fn [<test_matrix_norm_ $type _to_ $otype>]() {
                    let mut block = VectorNormBlock::<Matrix<2, 2, $type>, $otype>::default();
                    let p = Parameters::new();
                    let c = StubContext::default();

                    let input = Matrix {
                        data: [[3 as $type, 3 as $type], [3 as $type, 3 as $type]],
                    };
                    let output = block.process(&p, &c, &input);
                    assert_eq!(output, 6 as $otype);
                    assert_eq!(block.buffer(), output);
                }
            }
        };
    }

    test_vector_norm!(
        i8: f32, i8: f64,
        u8: f32, u8: f64,
        i16: f32, i16: f64,
        u16: f32, u16: f64,
        i32: f32, i32: f64,
        u32: f32, u32: f64,
        i64: f32, i64: f64,
        u64: f32, u64: f64,
        f32: f32, f32: f64,
        f64: f32, f64: f64
    );
}
