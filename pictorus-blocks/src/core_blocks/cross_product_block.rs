use crate::traits::Scalar;
use core::ops::{Mul, Sub};
use pictorus_traits::{Context, Matrix, Pass, PassBy};

pub struct Parameters {
    // No parameters needed for this block
}

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

/// Performs the cross product of two 3D vectors, either 1x3 or 3x1.
pub struct CrossProductBlock<T>
where
    T: Apply + Pass + Default,
{
    buffer: T::Output,
}

impl<T> Default for CrossProductBlock<T>
where
    T: Apply + Pass + Default,
{
    fn default() -> Self {
        Self {
            buffer: T::Output::default(),
        }
    }
}

impl<T> pictorus_traits::ProcessBlock for CrossProductBlock<T>
where
    T: Pass + Apply + Default,
{
    type Inputs = T;
    type Output = T::Output;
    type Parameters = Parameters;

    fn process<'b>(
        &'b mut self,
        _parameters: &Self::Parameters,
        _context: &dyn Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        let output = T::apply(&mut self.buffer, inputs);
        output
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        self.buffer.as_by()
    }
}

pub trait Apply: Pass + Sized {
    type Output: Pass + Default;

    fn apply<'s>(store: &'s mut Self::Output, inputs: PassBy<'_, Self>)
        -> PassBy<'s, Self::Output>;
}

/// Cross product over the flattened elements of two 3-element vectors.
/// Both 1x3 and 3x1 matrices store their elements contiguously in the same order.
fn cross3<S>(a: &[S], b: &[S], out: &mut [S])
where
    S: Scalar + Mul<Output = S> + Sub<Output = S>,
{
    out[0] = a[1] * b[2] - a[2] * b[1];
    out[1] = a[2] * b[0] - a[0] * b[2];
    out[2] = a[0] * b[1] - a[1] * b[0];
}

macro_rules! impl_cross_product {
    ($nrows:literal, $ncols:literal) => {
        impl<S> Apply for (Matrix<$nrows, $ncols, S>, Matrix<$nrows, $ncols, S>)
        where
            S: Scalar + Mul<Output = S> + Sub<Output = S>,
        {
            type Output = Matrix<$nrows, $ncols, S>;

            fn apply<'s>(
                store: &'s mut Self::Output,
                inputs: PassBy<'_, Self>,
            ) -> PassBy<'s, Self::Output> {
                cross3(
                    inputs.0.data.as_flattened(),
                    inputs.1.data.as_flattened(),
                    store.data.as_flattened_mut(),
                );
                store
            }
        }
    };
}

impl_cross_product!(1, 3);
impl_cross_product!(3, 1);

#[cfg(test)]
mod tests {

    use crate::testing::StubContext;
    use paste::paste;
    use pictorus_traits::ProcessBlock;

    use super::*;

    #[test]
    fn test_cross_product_default_buffer_no_panic() {
        let block = CrossProductBlock::<(Matrix<1, 3, f64>, Matrix<1, 3, f64>)>::default();
        assert_eq!(block.buffer(), &Matrix::<1, 3, f64>::zeroed());
    }

    macro_rules! test_cross_product {
        // Convenience call to generate a call to the main macro for every type in the list
        ($type:ty, $($other_types:ty),*) => {
            test_cross_product!($type);
            test_cross_product!($($other_types),*);
        };
        ($type:ty) => {
            paste! {
                #[test]
                fn [<test_vector_cross_unit_1x3_ $type>]() {
                    // x cross y = z, no negative intermediates so valid for unsigned types
                    let context = StubContext::default();
                    let p = Parameters::new();
                    let mut cross_block =
                        CrossProductBlock::<(Matrix<1, 3, $type>, Matrix<1, 3, $type>)>::default();
                    let input1: Matrix<1, 3, $type> = Matrix {
                        data: [[1 as $type], [0 as $type], [0 as $type]],
                    };
                    let input2: Matrix<1, 3, $type> = Matrix {
                        data: [[0 as $type], [1 as $type], [0 as $type]],
                    };
                    let output = cross_block.process(&p, &context, (&input1, &input2));
                    assert_eq!(output.data, [[0 as $type], [0 as $type], [1 as $type]]);
                    assert_eq!(cross_block.buffer().data, [[0 as $type], [0 as $type], [1 as $type]]);
                }

                #[test]
                fn [<test_vector_cross_unit_3x1_ $type>]() {
                    let context = StubContext::default();
                    let p = Parameters::new();
                    let mut cross_block =
                        CrossProductBlock::<(Matrix<3, 1, $type>, Matrix<3, 1, $type>)>::default();
                    let input1: Matrix<3, 1, $type> = Matrix {
                        data: [[1 as $type, 0 as $type, 0 as $type]],
                    };
                    let input2: Matrix<3, 1, $type> = Matrix {
                        data: [[0 as $type, 1 as $type, 0 as $type]],
                    };
                    let output = cross_block.process(&p, &context, (&input1, &input2));
                    assert_eq!(output.data, [[0 as $type, 0 as $type, 1 as $type]]);
                    assert_eq!(cross_block.buffer().data, [[0 as $type, 0 as $type, 1 as $type]]);
                }
            }
        };
    }

    test_cross_product!(f32, f64, i8, i16, i32, i64, u8, u16, u32, u64);

    macro_rules! test_cross_product_signed {
        // Convenience call to generate a call to the main macro for every type in the list
        ($type:ty, $($other_types:ty),*) => {
            test_cross_product_signed!($type);
            test_cross_product_signed!($($other_types),*);
        };
        ($type:ty) => {
            paste! {
                #[test]
                fn [<test_vector_cross_signed_ $type>]() {
                    // (2,3,4) cross (5,6,7) = (-3, 6, -3)
                    let context = StubContext::default();
                    let p = Parameters::new();
                    let mut cross_block =
                        CrossProductBlock::<(Matrix<1, 3, $type>, Matrix<1, 3, $type>)>::default();
                    let input1: Matrix<1, 3, $type> = Matrix {
                        data: [[2 as $type], [3 as $type], [4 as $type]],
                    };
                    let input2: Matrix<1, 3, $type> = Matrix {
                        data: [[5 as $type], [6 as $type], [7 as $type]],
                    };
                    let output = cross_block.process(&p, &context, (&input1, &input2));
                    assert_eq!(
                        output.data,
                        [[-3 as $type], [6 as $type], [-3 as $type]]
                    );
                }
            }
        };
    }

    test_cross_product_signed!(f32, f64, i8, i16, i32, i64);

    #[test]
    #[should_panic]
    fn multiplication_overflow_panics() {
        // The first output element computes 16 * 16 = 256, which overflows i8 in the
        // multiply step (before any subtraction). Native arithmetic panics in debug
        // builds (wraps in release).
        let context = StubContext::default();
        let p = Parameters::new();
        let mut cross_block = CrossProductBlock::<(Matrix<1, 3, i8>, Matrix<1, 3, i8>)>::default();
        let input1: Matrix<1, 3, i8> = Matrix {
            data: [[0], [16], [0]],
        };
        let input2: Matrix<1, 3, i8> = Matrix {
            data: [[0], [0], [16]],
        };
        let output = cross_block.process(&p, &context, (&input1, &input2));
        assert_eq!(output.data, [[0], [0], [0]]);
    }

    #[test]
    #[should_panic]
    fn underflow_panics_unsigned() {
        // y cross x = -z, which underflows unsigned types and panics in debug builds
        // (wraps in release).
        let context = StubContext::default();
        let p = Parameters::new();
        let mut cross_block = CrossProductBlock::<(Matrix<1, 3, u8>, Matrix<1, 3, u8>)>::default();
        let input1: Matrix<1, 3, u8> = Matrix {
            data: [[0], [1], [0]],
        };
        let input2: Matrix<1, 3, u8> = Matrix {
            data: [[1], [0], [0]],
        };
        let output = cross_block.process(&p, &context, (&input1, &input2));
        assert_eq!(output.data, [[0], [0], [u8::MAX]]);
    }
}
