use crate::nalgebra_interop::MatrixExt;
use crate::traits::{Apply, ApplyInto, MatrixOps, Scalar};
use block_data::{BlockData as OldBlockData, FromPass};
use nalgebra::SMatrix;
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock};

#[derive(strum::EnumString, Copy, Clone)]
pub enum MinMaxMethod {
    Min,
    Max,
}

pub struct Parameters {
    // The method to use for the MinMaxBlock. Must be either "Min" or "Max"
    pub method: MinMaxMethod,
}

impl Parameters {
    pub fn new(method: &str) -> Self {
        Parameters {
            method: method.parse().expect("Invalid method, must be Min or Max"),
        }
    }
}

/// A block that calculates the minimum or maximum of the input data
/// If inputs are all scalars, the output will be a scalar
/// Otherwise the output will be the component-wise minimum or maximum of the inputs
pub struct MinMaxBlock<T: Apply<Parameters>>
where
    OldBlockData: FromPass<<T as Apply<Parameters>>::Output>,
{
    pub data: OldBlockData,
    buffer: Option<T::Output>,
}

impl<T: Apply<Parameters>> Default for MinMaxBlock<T>
where
    OldBlockData: FromPass<<T as Apply<Parameters>>::Output>,
{
    fn default() -> Self {
        MinMaxBlock {
            data: <OldBlockData as FromPass<T::Output>>::from_pass(T::Output::default().as_by()),
            buffer: None,
        }
    }
}

impl<T: Apply<Parameters>> ProcessBlock for MinMaxBlock<T>
where
    OldBlockData: FromPass<T::Output>,
{
    type Parameters = Parameters;
    type Inputs = T;
    type Output = T::Output;

    fn process(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<Self::Output> {
        let res = T::apply(inputs, parameters, &mut self.buffer);
        self.data = OldBlockData::from_pass(res);
        res
    }
}

// Compare scalar with scalar
impl<S: Scalar> ApplyInto<S, Parameters> for S
where
    S: PartialOrd,
{
    fn apply_into<'a>(
        input: PassBy<Self>,
        params: &Parameters,
        dest: &'a mut Option<S>,
    ) -> PassBy<'a, S> {
        match dest {
            Some(dest) => match params.method {
                MinMaxMethod::Min => {
                    if input < *dest {
                        *dest = input;
                    }
                }
                MinMaxMethod::Max => {
                    if input > *dest {
                        *dest = input;
                    }
                }
            },
            None => {
                *dest = Some(input);
            }
        }

        dest.as_ref().unwrap().as_by()
    }
}

// Compare matrix and matrix
impl<const R: usize, const C: usize, S: Scalar> ApplyInto<Matrix<R, C, S>, Parameters>
    for Matrix<R, C, S>
{
    fn apply_into<'a>(
        input: PassBy<Self>,
        params: &Parameters,
        dest: &'a mut Option<Matrix<R, C, S>>,
    ) -> PassBy<'a, Matrix<R, C, S>> {
        match dest {
            Some(dest) => {
                let orig_dest = dest.as_view();
                let input = input.as_view();
                let res = match params.method {
                    MinMaxMethod::Min => input.inf(&orig_dest),
                    MinMaxMethod::Max => input.sup(&orig_dest),
                };
                dest.as_view_mut().copy_from(&res);
            }
            None => {
                *dest = Some(*input);
            }
        }

        dest.as_ref().unwrap().as_by()
    }
}

// Compare scalar with matrix
impl<const R: usize, const C: usize, S: Scalar> ApplyInto<Matrix<R, C, S>, Parameters> for S {
    fn apply_into<'a>(
        input: PassBy<Self>,
        params: &Parameters,
        dest: &'a mut Option<Matrix<R, C, S>>,
    ) -> PassBy<'a, Matrix<R, C, S>> {
        match dest {
            Some(dest) => {
                let orig_dest = dest.as_view();
                let input = SMatrix::<S, R, C>::from_element(input);
                let res = match params.method {
                    MinMaxMethod::Min => orig_dest.inf(&input.as_view()),
                    MinMaxMethod::Max => orig_dest.sup(&input.as_view()),
                };
                dest.as_view_mut().copy_from(&res);
            }
            None => {
                *dest = Some(Matrix::<R, C, S>::from_element(input));
            }
        }

        dest.as_ref().unwrap().as_by()
    }
}

#[cfg(test)]
mod tests {

    use crate::testing::StubContext;

    use super::*;

    #[test]
    fn test_single_scalar() {
        let ctxt = StubContext::default();
        let mut block = MinMaxBlock::<f64>::default();
        let mut parameters = Parameters::new("Min");
        let input = 99.0;
        let res = block.process(&parameters, &ctxt, input);
        assert_eq!(res, 99.0);
        assert_eq!(block.data.scalar(), 99.0);

        parameters.method = MinMaxMethod::Max;
        let res = block.process(&parameters, &ctxt, input.as_by());
        assert_eq!(res, 99.0);
        assert_eq!(block.data.scalar(), 99.0);
    }

    #[test]
    fn test_single_matrix() {
        let ctxt = StubContext::default();
        let mut block = MinMaxBlock::<Matrix<2, 2, f64>>::default();
        let mut parameters = Parameters::new("Min");
        let input = Matrix::<2, 2, f64>::from_element(99.0);
        let res = block.process(&parameters, &ctxt, &input);
        assert_eq!(res.data.as_flattened(), [99.0, 99.0, 99.0, 99.0]);
        assert_eq!(block.data.get_data().as_slice(), [99.0, 99.0, 99.0, 99.0]);

        parameters.method = MinMaxMethod::Max;
        let res = block.process(&parameters, &ctxt, &input);
        assert_eq!(res.data.as_flattened(), [99.0, 99.0, 99.0, 99.0]);
        assert_eq!(block.data.get_data().as_slice(), [99.0, 99.0, 99.0, 99.0]);
    }

    #[test]
    fn test_multiple_scalars() {
        let ctxt = StubContext::default();

        // Two inputs
        let mut two_block = MinMaxBlock::<(f64, f64)>::default();
        let mut parameters = Parameters::new("Min");
        let input = (99.0, 100.0);
        let res = two_block.process(&parameters, &ctxt, input);
        assert_eq!(res, 99.0);
        assert_eq!(two_block.data.scalar(), 99.0);

        parameters.method = MinMaxMethod::Max;
        let res = two_block.process(&parameters, &ctxt, input);
        assert_eq!(res, 100.0);
        assert_eq!(two_block.data.scalar(), 100.0);

        // Three inputs
        parameters.method = MinMaxMethod::Min;
        let mut three_block = MinMaxBlock::<(f64, f64, f64)>::default();
        let input = (99.0, 100.0, 101.0);
        let res = three_block.process(&parameters, &ctxt, input);
        assert_eq!(res, 99.0);
        assert_eq!(three_block.data.scalar(), 99.0);

        parameters.method = MinMaxMethod::Max;
        let res = three_block.process(&parameters, &ctxt, input);
        assert_eq!(res, 101.0);
        assert_eq!(three_block.data.scalar(), 101.0);

        // Four inputs
        parameters.method = MinMaxMethod::Min;
        let mut four_block = MinMaxBlock::<(f64, f64, f64, f64)>::default();
        let input = (99.0, 100.0, 101.0, 102.0);
        let res = four_block.process(&parameters, &ctxt, input);
        assert_eq!(res, 99.0);
        assert_eq!(four_block.data.scalar(), 99.0);

        parameters.method = MinMaxMethod::Max;
        let res = four_block.process(&parameters, &ctxt, input);
        assert_eq!(res, 102.0);
        assert_eq!(four_block.data.scalar(), 102.0);

        // Five inputs
        parameters.method = MinMaxMethod::Min;
        let mut five_block = MinMaxBlock::<(f64, f64, f64, f64, f64)>::default();
        let input = (99.0, 100.0, 101.0, 102.0, 103.0);
        let res = five_block.process(&parameters, &ctxt, input);
        assert_eq!(res, 99.0);
        assert_eq!(five_block.data.scalar(), 99.0);

        parameters.method = MinMaxMethod::Max;
        let res = five_block.process(&parameters, &ctxt, input);
        assert_eq!(res, 103.0);
        assert_eq!(five_block.data.scalar(), 103.0);

        // Six inputs
        parameters.method = MinMaxMethod::Min;
        let mut six_block = MinMaxBlock::<(f64, f64, f64, f64, f64, f64)>::default();
        let input = (99.0, 100.0, 101.0, 102.0, 103.0, 104.0);
        let res = six_block.process(&parameters, &ctxt, input);
        assert_eq!(res, 99.0);
        assert_eq!(six_block.data.scalar(), 99.0);

        parameters.method = MinMaxMethod::Max;
        let res = six_block.process(&parameters, &ctxt, input);
        assert_eq!(res, 104.0);
        assert_eq!(six_block.data.scalar(), 104.0);

        // Seven inputs
        parameters.method = MinMaxMethod::Min;
        let mut seven_block = MinMaxBlock::<(f64, f64, f64, f64, f64, f64, f64)>::default();
        let input = (99.0, 100.0, 101.0, 102.0, 103.0, 104.0, 105.0);
        let res = seven_block.process(&parameters, &ctxt, input);
        assert_eq!(res, 99.0);
        assert_eq!(seven_block.data.scalar(), 99.0);

        parameters.method = MinMaxMethod::Max;
        let res = seven_block.process(&parameters, &ctxt, input);
        assert_eq!(res, 105.0);
        assert_eq!(seven_block.data.scalar(), 105.0);

        // Eight inputs
        parameters.method = MinMaxMethod::Min;
        let mut eight_block = MinMaxBlock::<(f64, f64, f64, f64, f64, f64, f64, f64)>::default();
        let input = (99.0, 100.0, 101.0, 102.0, 103.0, 104.0, 105.0, 106.0);
        let res = eight_block.process(&parameters, &ctxt, input);
        assert_eq!(res, 99.0);
        assert_eq!(eight_block.data.scalar(), 99.0);

        parameters.method = MinMaxMethod::Max;
        let res = eight_block.process(&parameters, &ctxt, input);
        assert_eq!(res, 106.0);
        assert_eq!(eight_block.data.scalar(), 106.0);
    }

    #[test]
    fn test_multiple_matrices() {
        let ctxt = StubContext::default();

        // Two inputs
        let mut two_block = MinMaxBlock::<(Matrix<2, 2, f64>, Matrix<2, 2, f64>)>::default();
        let mut parameters = Parameters::new("Min");
        let input = (
            &Matrix {
                data: [[1.0, 6.0], [3.0, 8.0]],
            },
            &Matrix {
                data: [[5.0, 2.0], [7.0, 4.0]],
            },
        );
        let res = two_block.process(&parameters, &ctxt, input);
        assert_eq!(res.data.as_flattened(), [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(two_block.data.get_data().as_slice(), [1.0, 2.0, 3.0, 4.0]);

        parameters.method = MinMaxMethod::Max;
        let res = two_block.process(&parameters, &ctxt, input);
        assert_eq!(res.data.as_flattened(), [5.0, 6.0, 7.0, 8.0]);
        assert_eq!(two_block.data.get_data().as_slice(), [5.0, 6.0, 7.0, 8.0]);

        // Three inputs
        parameters.method = MinMaxMethod::Min;
        let mut three_block =
            MinMaxBlock::<(Matrix<2, 2, f64>, Matrix<2, 2, f64>, Matrix<2, 2, f64>)>::default();
        let input = (
            &Matrix {
                data: [[1.0, 6.0], [3.0, 8.0]],
            },
            &Matrix {
                data: [[5.0, 2.0], [7.0, 4.0]],
            },
            &Matrix {
                data: [[9.0, 10.0], [11.0, 12.0]],
            },
        );
        let res = three_block.process(&parameters, &ctxt, input);
        assert_eq!(res.data.as_flattened(), [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(three_block.data.get_data().as_slice(), [1.0, 2.0, 3.0, 4.0]);

        parameters.method = MinMaxMethod::Max;
        let res = three_block.process(&parameters, &ctxt, input);
        assert_eq!(res.data.as_flattened(), [9.0, 10.0, 11.0, 12.0]);
        assert_eq!(
            three_block.data.get_data().as_slice(),
            [9.0, 10.0, 11.0, 12.0]
        );

        // Four inputs
        parameters.method = MinMaxMethod::Min;
        let mut four_block = MinMaxBlock::<(
            Matrix<2, 2, f64>,
            Matrix<2, 2, f64>,
            Matrix<2, 2, f64>,
            Matrix<2, 2, f64>,
        )>::default();
        let input = (
            &Matrix {
                data: [[1.0, 6.0], [3.0, 8.0]],
            },
            &Matrix {
                data: [[5.0, 2.0], [7.0, 4.0]],
            },
            &Matrix {
                data: [[9.0, 10.0], [11.0, 12.0]],
            },
            &Matrix {
                data: [[13.0, 14.0], [15.0, 16.0]],
            },
        );
        let res = four_block.process(&parameters, &ctxt, input);
        assert_eq!(res.data.as_flattened(), [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(four_block.data.get_data().as_slice(), [1.0, 2.0, 3.0, 4.0]);

        parameters.method = MinMaxMethod::Max;
        let res = four_block.process(&parameters, &ctxt, input);
        assert_eq!(res.data.as_flattened(), [13.0, 14.0, 15.0, 16.0]);
        assert_eq!(
            four_block.data.get_data().as_slice(),
            [13.0, 14.0, 15.0, 16.0]
        );
    }

    #[test]
    fn test_mixed_scalars_and_matrices() {
        let ctxt = StubContext::default();

        // Scalar and matrix
        let mut block = MinMaxBlock::<(f64, Matrix<2, 2, f64>)>::default();
        let mut parameters = Parameters::new("Min");
        let input = (99.0, &Matrix::from_element(1.0));
        let res = block.process(&parameters, &ctxt, input);
        assert_eq!(res.data.as_flattened(), [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(block.data.get_data().as_slice(), [1.0, 1.0, 1.0, 1.0]);

        parameters.method = MinMaxMethod::Max;
        let res = block.process(&parameters, &ctxt, input);
        assert_eq!(res.data.as_flattened(), [99.0, 99.0, 99.0, 99.0]);
        assert_eq!(block.data.get_data().as_slice(), [99.0, 99.0, 99.0, 99.0]);

        // Matrix and scalar
        let mut block = MinMaxBlock::<(Matrix<2, 2, f64>, f64)>::default();
        let mut parameters = Parameters::new("Min");
        let input = (&Matrix::from_element(1.0), 99.0);
        let res = block.process(&parameters, &ctxt, input);
        assert_eq!(res.data.as_flattened(), [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(block.data.get_data().as_slice(), [1.0, 1.0, 1.0, 1.0]);

        parameters.method = MinMaxMethod::Max;
        let res = block.process(&parameters, &ctxt, input);
        assert_eq!(res.data.as_flattened(), [99.0, 99.0, 99.0, 99.0]);
        assert_eq!(block.data.get_data().as_slice(), [99.0, 99.0, 99.0, 99.0]);

        // (Scalar, matrix, scalar)
        let mut block = MinMaxBlock::<(f64, Matrix<2, 2, f64>, f64)>::default();
        let mut parameters = Parameters::new("Min");
        let input = (99.0, &Matrix::from_element(1.0), 100.0);
        let res = block.process(&parameters, &ctxt, input);
        assert_eq!(res.data.as_flattened(), [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(block.data.get_data().as_slice(), [1.0, 1.0, 1.0, 1.0]);

        parameters.method = MinMaxMethod::Max;
        let res = block.process(&parameters, &ctxt, input);
        assert_eq!(res.data.as_flattened(), [100.0, 100.0, 100.0, 100.0]);
        assert_eq!(
            block.data.get_data().as_slice(),
            [100.0, 100.0, 100.0, 100.0]
        );

        // (Matrix, scalar, matrix)
        let mut block = MinMaxBlock::<(Matrix<2, 2, f64>, f64, Matrix<2, 2, f64>)>::default();
        let mut parameters = Parameters::new("Min");
        let input = (&Matrix::from_element(1.0), 99.0, &Matrix::from_element(2.0));
        let res = block.process(&parameters, &ctxt, input);
        assert_eq!(res.data.as_flattened(), [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(block.data.get_data().as_slice(), [1.0, 1.0, 1.0, 1.0]);

        parameters.method = MinMaxMethod::Max;
        let res = block.process(&parameters, &ctxt, input);
        assert_eq!(res.data.as_flattened(), [99.0, 99.0, 99.0, 99.0]);
        assert_eq!(block.data.get_data().as_slice(), [99.0, 99.0, 99.0, 99.0]);
    }
}
