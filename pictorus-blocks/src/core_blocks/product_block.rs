use core::marker::PhantomData;
use pictorus_traits::{Pass, PassBy, ProcessBlock};

// This Block is essentially two blocks hiding in a trench coat; Matrix Multiplication and Component Wise Multiplication.
// Functionality for each has been broken out into separate modules to keep file sizes in check.
mod component;
use component::ApplyComponentWise;
mod matrix;
use matrix::{ApplyMatMul, ParametersMatrixMult};

/// Calculates the product of all of its input signals.
///
/// The product can be calculated in two ways:
/// - ComponentWise: Accepts Scalars, Same Size Matrices, or Scalars and Same Size Matrices
/// - MatrixMultiply: Accepts all matrices, using standard matrix multiplication sizing rules (i.e. (A, B) * (B, C) = (A, C))
pub struct ProductBlock<T: Apply<M>, M: ProductMethod> {
    _method: PhantomData<M>,
    store: T::Output,
}

impl<T: Apply<M>, M: ProductMethod> Default for ProductBlock<T, M> {
    fn default() -> Self {
        Self {
            _method: PhantomData,
            store: T::Output::default(),
        }
    }
}

impl<T: Apply<M>, M: ProductMethod> ProcessBlock for ProductBlock<T, M> {
    type Inputs = T;
    type Output = T::Output;
    type Parameters = T::Parameters;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        let mut tmp: Option<T::Output> = None;
        T::apply(&mut tmp, parameters, inputs);
        self.store = tmp.expect("apply must initialize the buffer");
        self.store.as_by()
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        self.store.as_by()
    }
}

/// This trait is what allows us to use either ComponentWise or MatrixMultiply
/// as the method for the ProductBlock.
pub trait Apply<M: ProductMethod>: Pass {
    type Output: Pass + Default;
    type Parameters;
    fn apply<'a>(
        buffer: &'a mut Option<Self::Output>,
        parameters: &Self::Parameters,
        inputs: PassBy<Self>,
    ) -> PassBy<'a, Self::Output>;
}

impl<T: ApplyMatMul> Apply<MatrixMultiply> for T {
    type Output = T::Output;
    type Parameters = ParametersMatrixMult;
    fn apply<'a>(
        buffer: &'a mut Option<Self::Output>,
        _parameters: &Self::Parameters,
        inputs: PassBy<Self>,
    ) -> PassBy<'a, Self::Output> {
        T::mat_mul(inputs, buffer)
    }
}

impl<T: ApplyComponentWise> Apply<ComponentWise> for T {
    type Output = T::Output;
    type Parameters = T::Parameters;
    fn apply<'a>(
        buffer: &'a mut Option<Self::Output>,
        parameters: &Self::Parameters,
        inputs: PassBy<Self>,
    ) -> PassBy<'a, Self::Output> {
        *buffer = None; // Reset Dest as None
        T::apply(inputs, parameters, buffer)
    }
}

/// This trait is used as a marker for the two different methods of product calculation.
pub trait ProductMethod {}
/// Calculate the product of all input signals component-wise.
pub struct ComponentWise;
impl ProductMethod for ComponentWise {}

/// Calculate the product of all input signals using matrix multiplication.
pub struct MatrixMultiply;
impl ProductMethod for MatrixMultiply {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use component::ParametersComponentWise;
    use paste::paste;
    use pictorus_traits::Matrix;

    #[test]
    fn test_product_default_buffer_no_panic() {
        let block = ProductBlock::<(f64, f64), ComponentWise>::default();
        assert_eq!(block.buffer(), 0.0);
    }

    #[test]
    fn test_component_wise_scalar() {
        let context = StubContext::default();

        // Scalars only
        let mut block = ProductBlock::<(f64, f64), ComponentWise>::default();
        let parameters =
            <ProductBlock<(f64, f64), ComponentWise> as ProcessBlock>::Parameters::new([1.0, 1.0]);
        let output = block.process(&parameters, &context, (11.0, 2.0));

        assert_eq!(output, 22.0);
        assert_eq!(block.buffer(), output);

        let mut block = ProductBlock::<(f32, f32, f32, f32, f32), ComponentWise>::default();
        let parameters = ParametersComponentWise::new([1.0, 1.0, 1.0, -1.0, 1.0]);
        let output = block.process(&parameters, &context, (11.0, 2.0, 3.0, 4.0, 5.0));

        assert_eq!(output, 82.5);
    }
    #[test]
    fn test_component_wise_scalar_matrix_mixed() {
        let context = StubContext::default();

        // Mixed Scalars and Matrices
        let mut block = ProductBlock::<(f64, Matrix<2, 2, f64>, f64), ComponentWise>::default();
        let parameters = ParametersComponentWise::new([1.0, 1.0, 1.0]);
        let output = block.process(
            &parameters,
            &context,
            (
                11.0,
                &Matrix {
                    data: [[1.0, 2.0], [3.0, 4.0]],
                },
                1.0,
            ),
        );
        let expected = Matrix {
            data: [[11.0, 22.0], [33.0, 44.0]],
        };

        assert_eq!(output, &expected);
        assert_eq!(block.buffer(), &expected);
    }

    #[test]
    fn test_component_wise_matrix() {
        let context = StubContext::default();

        // Matrices only
        let mut block =
            ProductBlock::<(Matrix<2, 2, f64>, Matrix<2, 2, f64>), ComponentWise>::default();
        let parameters =
            <ProductBlock<(Matrix<2, 2, f64>, Matrix<2, 2, f64>), ComponentWise> as ProcessBlock>::Parameters::new([1.0, 1.0]);
        let output = block.process(
            &parameters,
            &context,
            (
                &Matrix {
                    data: [[1.0, 2.0], [3.0, 4.0]],
                },
                &Matrix {
                    data: [[5.0, 6.0], [7.0, 8.0]],
                },
            ),
        );
        let expected = Matrix {
            data: [[5.0, 12.0], [21.0, 32.0]],
        };

        assert_eq!(output, &expected);
        assert_eq!(block.buffer(), &expected);
    }

    #[test]
    fn test_matrix_mult() {
        let context = StubContext::default();
        let p = ParametersMatrixMult {};

        let mut block =
            ProductBlock::<(Matrix<2, 2, f64>, Matrix<2, 2, f64>), MatrixMultiply>::default();
        let output = block.process(
            &p,
            &context,
            (
                &Matrix {
                    data: [[1.0, 3.0], [2.0, 4.0]],
                },
                &Matrix {
                    data: [[5.0, 7.0], [6.0, 8.0]],
                },
            ),
        );
        let expected = Matrix {
            data: [[19.0, 43.0], [22.0, 50.0]],
        };

        assert_eq!(output, &expected);
        assert_eq!(block.buffer(), &expected);

        let mut block = ProductBlock::<
            (Matrix<4, 2, f64>, Matrix<2, 3, f64>, Matrix<3, 2, f64>),
            MatrixMultiply,
        >::default();
        let output = block.process(
            &p,
            &context,
            (
                &Matrix {
                    data: [[1.0, 2.0, 3.0, 4.0], [5.0, 6.0, 7.0, 8.0]],
                },
                &Matrix {
                    data: [[5.0, 6.0], [7.0, 8.0], [9.0, 10.0]],
                },
                &Matrix {
                    data: [[42.0, 11.0, 12.0], [1337.0, 12.0, -4.0]],
                },
            ),
        );
        let expected = Matrix {
            data: [
                [2695.0, 3550.0, 4405.0, 5260.0],
                [47123.0, 61934.0, 76745.0, 91556.0],
            ],
        };

        assert_eq!(output, &expected);
        assert_eq!(block.buffer(), &expected);
    }

    macro_rules! test_product_types {
        // Convenience call to generate a call to the main macro for every type in the list
        ($type:ty, $($other_types:ty),*) => {
            test_product_types!($type);
            test_product_types!($($other_types),*);
        };
        ($type:ty) => {
            paste! {
                #[test]
                fn [<test_component_wise_scalar_ $type>]() {
                    let context = StubContext::default();
                    let mut block = ProductBlock::<($type, $type), ComponentWise>::default();

                    // Multiply only
                    let parameters = ParametersComponentWise::new([1.0, 1.0]);
                    let output = block.process(&parameters, &context, (6 as $type, 2 as $type));
                    assert_eq!(output, 12 as $type);
                    assert_eq!(block.buffer(), output);

                    // Multiply then divide, chosen to divide exactly for every type
                    let parameters = ParametersComponentWise::new([1.0, -1.0]);
                    let output = block.process(&parameters, &context, (12 as $type, 4 as $type));
                    assert_eq!(output, 3 as $type);
                    assert_eq!(block.buffer(), output);
                }

                #[test]
                fn [<test_component_wise_matrix_ $type>]() {
                    let context = StubContext::default();
                    let mut block = ProductBlock::<
                        (Matrix<2, 2, $type>, Matrix<2, 2, $type>),
                        ComponentWise,
                    >::default();
                    let parameters = ParametersComponentWise::new([1.0, 1.0]);
                    let output = block.process(
                        &parameters,
                        &context,
                        (
                            &Matrix {
                                data: [[1 as $type, 2 as $type], [3 as $type, 4 as $type]],
                            },
                            &Matrix {
                                data: [[5 as $type, 6 as $type], [7 as $type, 8 as $type]],
                            },
                        ),
                    );
                    let expected = Matrix {
                        data: [[5 as $type, 12 as $type], [21 as $type, 32 as $type]],
                    };
                    assert_eq!(output, &expected);
                    assert_eq!(block.buffer(), &expected);
                }

                #[test]
                fn [<test_component_wise_mixed_ $type>]() {
                    let context = StubContext::default();
                    let mut block =
                        ProductBlock::<($type, Matrix<2, 2, $type>), ComponentWise>::default();
                    let parameters = ParametersComponentWise::new([1.0, 1.0]);
                    let output = block.process(
                        &parameters,
                        &context,
                        (
                            3 as $type,
                            &Matrix {
                                data: [[1 as $type, 2 as $type], [3 as $type, 4 as $type]],
                            },
                        ),
                    );
                    let expected = Matrix {
                        data: [[3 as $type, 6 as $type], [9 as $type, 12 as $type]],
                    };
                    assert_eq!(output, &expected);
                    assert_eq!(block.buffer(), &expected);
                }
            }
        };
    }

    test_product_types!(f32, f64, i8, i16, i32, i64, u8, u16, u32, u64);

    #[test]
    fn test_matrix_mult_int() {
        let context = StubContext::default();
        let p = ParametersMatrixMult {};

        // [[1, 2, 3], [4, 5, 6]] * [[7, 8], [9, 10], [11, 12]] = [[58, 64], [139, 154]]
        let mut block =
            ProductBlock::<(Matrix<2, 3, i32>, Matrix<3, 2, i32>), MatrixMultiply>::default();
        let output = block.process(
            &p,
            &context,
            (
                &Matrix {
                    data: [[1, 4], [2, 5], [3, 6]],
                },
                &Matrix {
                    data: [[7, 9, 11], [8, 10, 12]],
                },
            ),
        );
        let expected = Matrix {
            data: [[58, 139], [64, 154]],
        };
        assert_eq!(output, &expected);
        assert_eq!(block.buffer(), &expected);

        // [[1, 2], [3, 4]] * [[5, 6], [7, 8]] = [[19, 22], [43, 50]], fits in u8
        let mut block =
            ProductBlock::<(Matrix<2, 2, u8>, Matrix<2, 2, u8>), MatrixMultiply>::default();
        let output = block.process(
            &p,
            &context,
            (
                &Matrix {
                    data: [[1u8, 3], [2, 4]],
                },
                &Matrix {
                    data: [[5u8, 7], [6, 8]],
                },
            ),
        );
        let expected = Matrix {
            data: [[19u8, 43], [22, 50]],
        };
        assert_eq!(output, &expected);
        assert_eq!(block.buffer(), &expected);
    }

    #[test]
    fn test_component_wise_int_division_truncates() {
        // Integer division truncates toward zero
        let context = StubContext::default();
        let mut block = ProductBlock::<(i32, i32), ComponentWise>::default();
        let parameters = ParametersComponentWise::new([1.0, -1.0]);
        let output = block.process(&parameters, &context, (7, 2));
        assert_eq!(output, 3);
    }

    #[test]
    fn test_component_wise_int_leading_divide() {
        // The accumulator starts at one, so a leading divide computes 1 / x,
        // which truncates to zero for any integer input > 1
        let context = StubContext::default();
        let mut block = ProductBlock::<(i32, i32), ComponentWise>::default();
        let parameters = ParametersComponentWise::new([-1.0, 1.0]);
        let output = block.process(&parameters, &context, (5, 10));
        assert_eq!(output, 0);
    }

    #[test]
    #[should_panic]
    fn int_division_by_zero_panics() {
        // Integer division by zero panics in all build profiles (unlike float inf)
        let context = StubContext::default();
        let mut block = ProductBlock::<(i32, i32), ComponentWise>::default();
        let parameters = ParametersComponentWise::new([1.0, -1.0]);
        let output = block.process(&parameters, &context, (4, 0));
        assert_eq!(output, 0);
    }

    #[test]
    #[should_panic]
    fn int_multiply_overflow_panics() {
        // 16 * 16 overflows u8; native arithmetic panics in debug builds (wraps in release).
        let context = StubContext::default();
        let mut block = ProductBlock::<(u8, u8), ComponentWise>::default();
        let parameters = ParametersComponentWise::new([1.0, 1.0]);
        let output = block.process(&parameters, &context, (16u8, 16u8));
        assert_eq!(output, 0);
    }

    #[test]
    #[should_panic]
    fn int_multiply_negative_overflow_panics() {
        // -100 * 2 = -200 underflows i8::MIN; native integer multiplication panics in
        // debug builds (wraps in release).
        let context = StubContext::default();
        let mut block = ProductBlock::<(i8, i8), ComponentWise>::default();
        let parameters = ParametersComponentWise::new([1.0, 1.0]);
        let output = block.process(&parameters, &context, (-100i8, 2i8));
        assert_eq!(output, 56);
    }

    #[test]
    #[should_panic]
    fn matrix_mult_overflow_panics() {
        // Each output element is 16*16 + 16*16 = 512, which overflows u8;
        // panics in debug builds (wraps in release).
        let context = StubContext::default();
        let p = ParametersMatrixMult {};
        let mut block =
            ProductBlock::<(Matrix<2, 2, u8>, Matrix<2, 2, u8>), MatrixMultiply>::default();
        let input = Matrix { data: [[16u8; 2]; 2] };
        let output = block.process(&p, &context, (&input, &input));
        assert_eq!(output, &Matrix { data: [[0u8; 2]; 2] });
    }
}
