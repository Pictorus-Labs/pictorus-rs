use core::ops::Sub;
use num_traits::One;
use pictorus_block_data::{BlockData as OldBlockData, FromPass};
#[cfg(feature = "tricore")]
use pictorus_traits::FlattenSlice as _;
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock};

use crate::traits::{Apply, ApplyInto, MatrixOps, Scalar};

/// Performs logical operations on inputs.
///
/// Currently supports the following methods:
/// - And
/// - Or
/// - Nor
/// - Nand
pub struct LogicalBlock<T>
where
    T: Apply<Parameters>,
    T::Output: Finalize,
    OldBlockData: FromPass<<T as Apply<Parameters>>::Output>,
{
    store: Option<T::Output>,
    pub data: OldBlockData,
}

impl<T> Default for LogicalBlock<T>
where
    T: Apply<Parameters>,
    T::Output: Finalize,
    OldBlockData: FromPass<<T as Apply<Parameters>>::Output>,
{
    fn default() -> Self {
        Self {
            store: None,
            data: <OldBlockData as FromPass<T::Output>>::from_pass(T::Output::default().as_by()),
        }
    }
}

impl<T> ProcessBlock for LogicalBlock<T>
where
    T: Apply<Parameters>,
    T::Output: Finalize,
    OldBlockData: FromPass<<T as Apply<Parameters>>::Output>,
{
    type Inputs = T;
    type Output = T::Output;
    type Parameters = Parameters;
    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        self.store = None;
        T::apply(inputs, parameters, &mut self.store);
        let result = T::Output::finalize(parameters.method, &mut self.store);
        self.data = OldBlockData::from_pass(result);
        result
    }
}

fn perform_op<S: Scalar + From<bool>>(input: S, dest: S, method: LogicalMethod) -> S {
    let x0 = input.is_truthy();
    let x1 = dest.is_truthy();
    let res = match method {
        LogicalMethod::And => x0 & x1,
        LogicalMethod::Or => x0 | x1,
        // NAND and NOR behave the same as OR and AND during
        // the calculation, but the final result is inverted in the finalize step
        LogicalMethod::Nand => x0 & x1,
        LogicalMethod::Nor => x0 | x1,
    };

    res.into()
}

// Compare scalar with scalar
impl<S: Scalar + From<bool>> ApplyInto<S, Parameters> for S {
    fn apply_into<'a>(
        input: PassBy<Self>,
        params: &Parameters,
        dest: &'a mut Option<S>,
    ) -> PassBy<'a, S> {
        match dest {
            Some(dest) => {
                *dest = perform_op(input, *dest, params.method);
            }
            None => {
                *dest = Some(input);
            }
        }

        dest.as_ref().unwrap().as_by()
    }
}

// Compare matrix and matrix
impl<const R: usize, const C: usize, S: Scalar + From<bool>> ApplyInto<Matrix<R, C, S>, Parameters>
    for Matrix<R, C, S>
{
    fn apply_into<'a>(
        input: PassBy<Self>,
        params: &Parameters,
        dest: &'a mut Option<Matrix<R, C, S>>,
    ) -> PassBy<'a, Matrix<R, C, S>> {
        match dest {
            Some(dest) => {
                input
                    .data
                    .as_flattened()
                    .iter()
                    .zip(dest.data.as_flattened_mut().iter_mut())
                    .for_each(|(input, dest)| {
                        *dest = perform_op(*input, *dest, params.method);
                    });
            }
            None => {
                *dest = Some(*input);
            }
        }

        dest.as_ref().unwrap().as_by()
    }
}

// Compare scalar with matrix
impl<const R: usize, const C: usize, S: Scalar + From<bool>> ApplyInto<Matrix<R, C, S>, Parameters>
    for S
{
    fn apply_into<'a>(
        input: PassBy<Self>,
        params: &Parameters,
        dest: &'a mut Option<Matrix<R, C, S>>,
    ) -> PassBy<'a, Matrix<R, C, S>> {
        match dest {
            Some(dest) => {
                dest.data.as_flattened_mut().iter_mut().for_each(|dest| {
                    *dest = perform_op(input, *dest, params.method);
                });
            }
            None => {
                *dest = Some(Matrix::<R, C, S>::from_element(input));
            }
        }

        dest.as_ref().unwrap().as_by()
    }
}

pub trait Finalize: Pass + Default {
    fn finalize(method: LogicalMethod, dest: &mut Option<Self>) -> PassBy<'_, Self>;
}

impl<S: Scalar + One + Sub<Output = S>> Finalize for S {
    fn finalize(method: LogicalMethod, dest: &mut Option<Self>) -> PassBy<'_, Self> {
        let input = dest.get_or_insert(S::default());
        let res = match method {
            LogicalMethod::Nor => S::one() - *input,
            LogicalMethod::Nand => S::one() - *input,
            _ => *input,
        };

        *dest = Some(res);
        dest.as_ref().unwrap().as_by()
    }
}

impl<const R: usize, const C: usize, S: Scalar + One + Sub<Output = S>> Finalize
    for Matrix<R, C, S>
{
    fn finalize(method: LogicalMethod, dest: &mut Option<Self>) -> PassBy<'_, Self> {
        let dest = dest.get_or_insert(Matrix::<R, C, S>::default());
        dest.data.as_flattened_mut().iter_mut().for_each(|dest| {
            *dest = match method {
                LogicalMethod::Nor => S::one() - *dest,
                LogicalMethod::Nand => S::one() - *dest,
                _ => *dest,
            };
        });

        dest.as_by()
    }
}

#[derive(Debug, Clone, Copy, strum::EnumString)]
/// Logical methods that can be applied
pub enum LogicalMethod {
    /// Logical AND operation
    And,
    /// Logical OR operation
    Or,
    /// Logical NOR operation (NOT OR)
    Nor,
    /// Logical NAND operation (NOT AND)
    Nand,
}

/// Parameters for the logical block
pub struct Parameters {
    /// The logical method to use
    method: LogicalMethod,
}

impl Parameters {
    pub fn new(method: &str) -> Self {
        Self {
            method: method.parse().expect("Failed to parse logical method."),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::StubContext;

    use super::*;

    #[test]
    fn test_logical_and_scalar() {
        let ctxt = StubContext::default();
        let params = Parameters::new("And");
        let mut block = LogicalBlock::<(f64, f64, f64)>::default();

        // All zero aka false inputs = false output
        let res = block.process(&params, &ctxt, (0.0, 0.0, 0.0));
        assert_eq!(res, 0.0);
        assert_eq!(block.data.scalar(), 0.0);

        // Some zero inputs = false output
        let res = block.process(&params, &ctxt, (1.0, 0.0, 1.0));
        assert_eq!(res, 0.0);
        assert_eq!(block.data.scalar(), 0.0);

        // All non-zero inputs = true output
        let res = block.process(&params, &ctxt, (1.0, 1.0, 1.0));
        assert_eq!(res, 1.0);
        assert_eq!(block.data.scalar(), 1.0);

        // Even floats and negative data!
        let res = block.process(&params, &ctxt, (1.0, -2.0, 3.5));
        assert_eq!(res, 1.0);
        assert_eq!(block.data.scalar(), 1.0);
    }

    #[test]
    fn test_logical_or_scalar() {
        let ctxt = StubContext::default();
        let params = Parameters::new("Or");
        let mut block = LogicalBlock::<(f64, f64, f64)>::default();

        // All zero aka false inputs = false output
        let res = block.process(&params, &ctxt, (0.0, 0.0, 0.0));
        assert_eq!(res, 0.0);
        assert_eq!(block.data.scalar(), 0.0);

        // Some zero inputs = true output
        let res = block.process(&params, &ctxt, (1.0, 0.0, 1.0));
        assert_eq!(res, 1.0);
        assert_eq!(block.data.scalar(), 1.0);

        // All non-zero inputs = true output
        let res = block.process(&params, &ctxt, (1.0, 1.0, 1.0));
        assert_eq!(res, 1.0);
        assert_eq!(block.data.scalar(), 1.0);

        // Even floats and negative data!
        let res = block.process(&params, &ctxt, (1.0, -2.0, 3.5));
        assert_eq!(res, 1.0);
        assert_eq!(block.data.scalar(), 1.0);
    }

    #[test]
    fn test_logical_nor_scalar() {
        let ctxt = StubContext::default();
        let params = Parameters::new("Nor");
        let mut block = LogicalBlock::<(f64, f64, f64)>::default();

        // These tests should be the opposite results of the OR tests

        // All zero aka false inputs = true output
        let res = block.process(&params, &ctxt, (0.0, 0.0, 0.0));
        assert_eq!(res, 1.0);
        assert_eq!(block.data.scalar(), 1.0);

        // Some zero inputs = false output
        let res = block.process(&params, &ctxt, (1.0, 0.0, 1.0));
        assert_eq!(res, 0.0);
        assert_eq!(block.data.scalar(), 0.0);

        // All non-zero inputs = false output
        let res = block.process(&params, &ctxt, (1.0, 1.0, 1.0));
        assert_eq!(res, 0.0);
        assert_eq!(block.data.scalar(), 0.0);

        // Even floats and negative data!
        let res = block.process(&params, &ctxt, (1.0, -2.0, 3.5));
        assert_eq!(res, 0.0);
        assert_eq!(block.data.scalar(), 0.0);
        assert_eq!(block.data.scalar(), 0.0);
    }

    #[test]
    fn test_logical_nand_scalar() {
        let ctxt = StubContext::default();
        let params = Parameters::new("Nand");
        let mut block = LogicalBlock::<(f64, f64, f64)>::default();

        // These tests should be the opposite results of the AND tests

        // All zero aka false inputs = true output
        let res = block.process(&params, &ctxt, (0.0, 0.0, 0.0));
        assert_eq!(res, 1.0);
        assert_eq!(block.data.scalar(), 1.0);

        // Some zero inputs = true output
        let res = block.process(&params, &ctxt, (1.0, 0.0, 1.0));
        assert_eq!(res, 1.0);
        assert_eq!(block.data.scalar(), 1.0);

        // All non-zero inputs = false output
        let res = block.process(&params, &ctxt, (1.0, 1.0, 1.0));
        assert_eq!(res, 0.0);
        assert_eq!(block.data.scalar(), 0.0);

        // Even floats and negative data!
        let res = block.process(&params, &ctxt, (1.0, -2.0, 3.5));
        assert_eq!(res, 0.0);
        assert_eq!(block.data.scalar(), 0.0);
    }

    #[test]
    fn test_matrix_ops() {
        let ctxt = StubContext::default();
        let mut params = Parameters::new("And");
        let mut block =
            LogicalBlock::<(Matrix<2, 2, f64>, Matrix<2, 2, f64>, Matrix<2, 2, f64>)>::default();

        let input = (
            &Matrix {
                data: [[1.0, 0.0], [0.0, 1.0]],
            },
            &Matrix {
                data: [[0.0, 1.0], [1.0, 0.0]],
            },
            &Matrix {
                data: [[1.0, 1.0], [1.0, 1.0]],
            },
        );

        let res = block.process(&params, &ctxt, input);
        let expected = Matrix {
            data: [[0.0, 0.0], [0.0, 0.0]],
        };
        assert_eq!(res, &expected);
        assert_eq!(
            block.data.get_data().as_slice(),
            expected.data.as_flattened()
        );

        params.method = LogicalMethod::Or;
        let res = block.process(&params, &ctxt, input);
        let expected = Matrix {
            data: [[1.0, 1.0], [1.0, 1.0]],
        };
        assert_eq!(res, &expected);
        assert_eq!(
            block.data.get_data().as_slice(),
            expected.data.as_flattened()
        );

        params.method = LogicalMethod::Nor;
        let res = block.process(&params, &ctxt, input);
        let expected = Matrix {
            data: [[0.0, 0.0], [0.0, 0.0]],
        };
        assert_eq!(res, &expected);
        assert_eq!(
            block.data.get_data().as_slice(),
            expected.data.as_flattened()
        );

        params.method = LogicalMethod::Nand;
        let res = block.process(&params, &ctxt, input);
        let expected = Matrix {
            data: [[1.0, 1.0], [1.0, 1.0]],
        };
        assert_eq!(res, &expected);
        assert_eq!(
            block.data.get_data().as_slice(),
            expected.data.as_flattened()
        );
    }

    #[test]
    fn test_matrix_scalar_ops() {
        let ctxt = StubContext::default();
        let mut params = Parameters::new("And");
        let mut block = LogicalBlock::<(Matrix<2, 2, f64>, f64)>::default();

        let input = (
            &Matrix {
                data: [[1.0, 0.0], [0.0, 1.0]],
            },
            1.0,
        );

        let res = block.process(&params, &ctxt, input);
        let expected = Matrix {
            data: [[1.0, 0.0], [0.0, 1.0]],
        };
        assert_eq!(res, &expected);
        assert_eq!(
            block.data.get_data().as_slice(),
            expected.data.as_flattened()
        );

        params.method = LogicalMethod::Or;
        let res = block.process(&params, &ctxt, input);
        let expected = Matrix {
            data: [[1.0, 1.0], [1.0, 1.0]],
        };
        assert_eq!(res, &expected);
        assert_eq!(
            block.data.get_data().as_slice(),
            expected.data.as_flattened()
        );

        params.method = LogicalMethod::Nor;
        let res = block.process(&params, &ctxt, input);
        let expected = Matrix {
            data: [[0.0, 0.0], [0.0, 0.0]],
        };
        assert_eq!(res, &expected);
        assert_eq!(
            block.data.get_data().as_slice(),
            expected.data.as_flattened()
        );

        params.method = LogicalMethod::Nand;
        let res = block.process(&params, &ctxt, input);
        let expected = Matrix {
            data: [[0.0, 1.0], [1.0, 0.0]],
        };
        assert_eq!(res, &expected);
        assert_eq!(
            block.data.get_data().as_slice(),
            expected.data.as_flattened()
        );
    }
}
