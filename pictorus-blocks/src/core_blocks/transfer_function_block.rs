use crate::traits::{Float, MatrixOps};
use heapless::Deque;
use num_traits::Zero;
use pictorus_block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock};

/// Parameters for the TransferFunctionBlock
pub struct Parameters<F: Float, const NUM_SIZE: usize, const DEN_SIZE: usize> {
    pub numerators: [F; NUM_SIZE],
    pub denominators: [F; DEN_SIZE],
}

impl<F: Float, const NUM_SIZE: usize, const DEN_SIZE: usize> Parameters<F, NUM_SIZE, DEN_SIZE> {
    pub fn new(numerators: &OldBlockData, denominators: &OldBlockData) -> Self {
        let mut l_numerators = [F::zero(); NUM_SIZE];
        let mut l_denominators = [F::zero(); DEN_SIZE];

        for (i, num) in numerators.iter().enumerate() {
            l_numerators[i] = F::from(*num).expect("Failed to convert numerator to Float");
        }

        for (i, den) in denominators.iter().enumerate() {
            l_denominators[i] = F::from(*den).expect("Failed to convert numerator to Float");
        }

        Parameters {
            numerators: l_numerators,
            denominators: l_denominators,
        }
    }

    pub fn new_arr(numerators: &[F], denominators: &[F]) -> Self {
        let mut l_numerators = [F::zero(); NUM_SIZE];
        let mut l_denominators = [F::zero(); DEN_SIZE];

        for (i, num) in numerators.iter().enumerate() {
            l_numerators[i] = F::from(*num).expect("Failed to convert numerator to Float");
        }

        for (i, den) in denominators.iter().enumerate() {
            l_denominators[i] = F::from(*den).expect("Failed to convert numerator to Float");
        }

        Parameters {
            numerators: l_numerators,
            denominators: l_denominators,
        }
    }
}

/// Implements a transfer function `H(z) = Y(z) / X(z)` over the input signal.
///
/// The parameters for the block require an array of numerator and denominator coefficients used to compute the
/// output of the block:
/// `y[n] = b[0] * x[n] + b[1] * x[n-1] + ... + b[n] * x[n-n] - a[1] * y[n-1] - a[2] * y[n-2] + ...`
///
/// A transfer function that represents an decay of 1/2 each sample can be represented as:
/// `H(z) = Y(z) / X(z) = z / (z - 0.5)`
///
/// The numerator would be `[1]` and the denominator would be `[1, -0.5]`.
///
/// The numerator and denominator must have dimensions of at least 1 and the 0th value of the
/// denominator will be skipped (but must still be present), as it represents the coefficient
/// for `y[n]`, the current output.
pub struct TransferFunctionBlock<const NUM_SIZE: usize, const DEN_SIZE: usize, F: Float, I>
where
    I: Default,
{
    pub data: OldBlockData,
    buffer: I,
    /// Stores input samples, typically denoted as x[n]
    input: Deque<I, NUM_SIZE>,
    /// Stores output samples, typically denoted as y[n]
    output: Deque<I, DEN_SIZE>,
    phantom: core::marker::PhantomData<F>,
}

impl<const NUM_SIZE: usize, const DEN_SIZE: usize, F, I> Default
    for TransferFunctionBlock<NUM_SIZE, DEN_SIZE, F, I>
where
    F: Float,
    I: Pass + Default,
    OldBlockData: FromPass<I>,
{
    fn default() -> Self {
        Self {
            data: <OldBlockData as FromPass<I>>::from_pass(<I>::default().as_by()),
            buffer: I::default(),
            input: Deque::new(),
            output: Deque::new(),
            phantom: core::marker::PhantomData,
        }
    }
}

macro_rules! impl_transfer_function {
    ($type:ty) => {
        impl<const NUM_SIZE: usize, const DEN_SIZE: usize> ProcessBlock
            for TransferFunctionBlock<NUM_SIZE, DEN_SIZE, $type, $type>
        where
            OldBlockData: FromPass<f64>,
        {
            type Inputs = $type;
            type Output = $type;
            type Parameters = Parameters<$type, NUM_SIZE, DEN_SIZE>;

            fn process(
                &mut self,
                parameters: &Self::Parameters,
                _context: &dyn pictorus_traits::Context,
                input: PassBy<Self::Inputs>,
            ) -> PassBy<'_, Self::Output> {
                if self.input.is_empty() {
                    for _ in 0..(NUM_SIZE - 1) {
                        self.input.push_front(<$type>::zero()).expect(
                            "Failed to push to samples when initializing TransferFunctionBlock",
                        );
                    }
                }

                if self.output.is_empty() {
                    for _ in 0..DEN_SIZE {
                        self.output.push_front(<$type>::zero()).expect(
                            "Failed to push to samples when initializing TransferFunctionBlock",
                        );
                    }
                }

                self.input
                    .push_front(input)
                    .expect("Failed to push to samples in TransferFunctionBlock");

                // as_mut_slices() seems to mess up the operation of the queue, clone it
                // on the stack and work with the clone
                let mut input_clone = self.input.clone();
                let (input_front, _) = input_clone.as_mut_slices();

                let mut output_clone = self.output.clone();
                let (output_front, _) = output_clone.as_mut_slices();

                // input_front at this point is x[n], x[n-1], x[n-2], ...
                let mut x_z = <$type>::zero();
                for (i, n) in parameters.numerators.iter().enumerate() {
                    x_z += *n * input_front[i];
                }

                // output_front at this point is y[n-1], y[n-2], y[n-3], ...
                // Skip the 0th element of the denominator BUT grab the
                // y[n-1] element when it is time to calculate y[n]
                let mut y_z = <$type>::zero();
                for (i, d) in parameters.denominators.iter().enumerate().skip(1) {
                    y_z -= *d * output_front[i - 1];
                }

                // y[n]
                self.buffer = x_z + y_z;

                self.output.pop_back();
                self.output
                    .push_front(self.buffer)
                    .expect("Failed to push to output sample in TransferFunctionBlock");

                self.input.pop_back();

                self.data = OldBlockData::from_scalar(self.buffer.into());
                self.buffer
            }
        }

        impl<
                const NUM_SIZE: usize,
                const DEN_SIZE: usize,
                const ROWS: usize,
                const COLS: usize,
            > ProcessBlock
            for TransferFunctionBlock<NUM_SIZE, DEN_SIZE, $type, Matrix<ROWS, COLS, $type>>
        where
            $type: Float,
            OldBlockData: FromPass<Matrix<ROWS, COLS, $type>>,
        {
            type Inputs = Matrix<ROWS, COLS, $type>;
            type Output = Matrix<ROWS, COLS, $type>;
            type Parameters = Parameters<$type, NUM_SIZE, DEN_SIZE>;

            fn process(
                &mut self,
                parameters: &Self::Parameters,
                _context: &dyn pictorus_traits::Context,
                input: PassBy<Self::Inputs>,
            ) -> PassBy<'_, Self::Output> {
                if self.input.is_empty() {
                    for _ in 0..(NUM_SIZE - 1) {
                        self.input.push_front(Matrix::zeroed()).expect(
                            "Failed to push to samples when initializing TransferFunctionBlock",
                        );
                    }
                }

                if self.output.is_empty() {
                    for _ in 0..DEN_SIZE {
                        self.output.push_front(Matrix::zeroed()).expect(
                            "Failed to push to samples when initializing TransferFunctionBlock",
                        );
                    }
                }

                self.input
                    .push_front(*input)
                    .expect("Failed to push to samples in TransferFunctionBlock");

                // as_mut_slices() seems to mess up the operation of the queue, clone it
                // on the stack and work with the clone
                let mut input_clone = self.input.clone();
                let (input_front, _) = input_clone.as_mut_slices();

                let mut output_clone = self.output.clone();
                let (output_front, _) = output_clone.as_mut_slices();

                let mut x_z = Matrix::zeroed();
                for (i, matrix) in input_front.iter().enumerate() {
                    matrix.for_each(|f, c, r| {
                        x_z.data[c][r] += parameters.numerators[i] * f;
                    });
                }

                let mut y_z = Matrix::<ROWS, COLS, $type>::zeroed();
                // output_front at this point is y[n-1], y[n-2], y[n-3], ...
                // Skip the 0th element of the denominator BUT grab the
                // y[n-1] element when it is time to calculate y[n]
                for (i, d) in parameters.denominators.iter().enumerate().skip(1) {
                    output_front[i - 1].for_each(|f, c, r| {
                        y_z.data[c][r] -= *d * f;
                    });
                }

                // y[n]
                self.buffer = x_z.map_collect(|f, c, r| f + y_z.data[c][r]);

                self.output.pop_back();
                self.output
                    .push_front(self.buffer)
                    .expect("Failed to push to output sample in TransferFunctionBlock");

                self.input.pop_back();

                self.data = OldBlockData::from_pass(self.buffer.as_by());
                &self.buffer
            }
        }
    };
}

impl_transfer_function!(f64);
impl_transfer_function!(f32);

#[cfg(test)]
mod tests {
    use super::Parameters;
    use crate::testing::StubContext;
    use approx::assert_relative_eq;
    use pictorus_block_data::BlockData;
    use pictorus_traits::{Matrix, ProcessBlock};

    use crate::TransferFunctionBlock;

    #[test]
    fn test_transfer_function_block_scalar_unity() {
        let c = StubContext::default();
        let num = [1.0];
        let denom = [1.0];
        let parameters = Parameters::new_arr(&num, &denom);

        let mut block = TransferFunctionBlock::<1, 1, f64, f64>::default();

        let output = block.process(&parameters, &c, 1.0);
        assert_eq!(output, 1.0);

        let output = block.process(&parameters, &c, 10.0);
        assert_relative_eq!(output, 10.0, max_relative = 0.01);

        let output = block.process(&parameters, &c, 1.0);
        assert_relative_eq!(output, 1.0, max_relative = 0.01);
    }

    #[test]
    fn test_transfer_function_block_scalar_delay() {
        let c = StubContext::default();
        let num = [0.0, 1.0];
        let denom = [1.0];

        let num = BlockData::from_vector(&num);
        let denom = BlockData::from_vector(&denom);

        let parameters = super::Parameters::new(&num, &denom);

        let mut block = TransferFunctionBlock::<2, 1, f64, f64>::default();

        let output = block.process(&parameters, &c, 1.0);
        assert_eq!(output, 0.0);

        let output = block.process(&parameters, &c, 10.0);
        assert_relative_eq!(output, 1.0, max_relative = 0.01);

        let output = block.process(&parameters, &c, 1.0);
        assert_relative_eq!(output, 10.0, max_relative = 0.01);
    }

    #[test]
    fn test_transfer_function_block_scalar_exp_decay() {
        // Divide by 2 each call to process
        let c = StubContext::default();
        let num = [1.0];
        let denom = [1.0, -0.5];
        let parameters = super::Parameters::new_arr(&num, &denom);

        let mut block = TransferFunctionBlock::<1, 2, f64, f64>::default();

        let output = block.process(&parameters, &c, 1.0);
        assert_eq!(output, 1.0);

        let output = block.process(&parameters, &c, 0.0);
        assert_relative_eq!(output, 0.5, max_relative = 0.01);

        let output = block.process(&parameters, &c, 0.0);
        assert_relative_eq!(output, 0.25, max_relative = 0.01);

        let output = block.process(&parameters, &c, 0.0);
        assert_relative_eq!(output, 0.125, max_relative = 0.01);
    }

    #[test]
    fn test_transfer_function_block_scalar_integrator() {
        let c = StubContext::default();
        let num = [1.0];
        let denom = [1.0, -1.0];
        let parameters = super::Parameters::new_arr(&num, &denom);

        let mut block = TransferFunctionBlock::<1, 2, f64, f64>::default();

        let output = block.process(&parameters, &c, 1.0);
        assert_eq!(output, 1.0);

        let output = block.process(&parameters, &c, 1.0);
        assert_relative_eq!(output, 2.0, max_relative = 0.01);

        let output = block.process(&parameters, &c, 1.0);
        assert_relative_eq!(output, 3.0, max_relative = 0.01);

        let output = block.process(&parameters, &c, 1.0);
        assert_relative_eq!(output, 4.0, max_relative = 0.01);
    }

    #[test]
    fn test_transfer_function_block_scalar_differentiator() {
        let c = StubContext::default();
        let num = [1.0, -1.0];
        let denom = [1.0];
        let parameters = super::Parameters::new_arr(&num, &denom);

        let mut block = TransferFunctionBlock::<2, 1, f64, f64>::default();

        let output = block.process(&parameters, &c, 1.0);
        assert_eq!(output, 1.0);

        let output = block.process(&parameters, &c, 2.0);
        assert_eq!(output, 1.0);

        let output = block.process(&parameters, &c, 3.0);
        assert_eq!(output, 1.0);

        let output = block.process(&parameters, &c, 4.0);
        assert_eq!(output, 1.0);

        let output = block.process(&parameters, &c, 10.0);
        assert_eq!(output, 6.0);
    }

    #[test]
    fn test_transfer_function_block_matrix_exp_decay() {
        // Divide by 2 each call to process
        let c = StubContext::default();
        let num = [1.0];
        let denom = [1.0, -0.5];
        let parameters = super::Parameters::new_arr(&num, &denom);

        let mut block = TransferFunctionBlock::<1, 2, f64, Matrix<2, 2, f64>>::default();

        let output = block.process(
            &parameters,
            &c,
            &Matrix {
                data: [[1.0, -1.0], [10.0, -10.0]],
            },
        );
        assert_eq!(
            output,
            &Matrix {
                data: [[1.0, -1.0], [10.0, -10.0]]
            }
        );

        let zeroed = Matrix::<2, 2, f64>::zeroed();
        let output = block.process(&parameters, &c, &zeroed);
        assert_relative_eq!(
            output.data.as_flattened(),
            &Matrix {
                data: [[0.5, -0.5], [5.0, -5.0]]
            }
            .data
            .as_flattened(),
            max_relative = 0.01
        );

        let output = block.process(&parameters, &c, &zeroed);
        assert_relative_eq!(
            output.data.as_flattened(),
            &Matrix {
                data: [[0.25, -0.25], [2.5, -2.5]]
            }
            .data
            .as_flattened(),
            max_relative = 0.01
        );

        let output = block.process(&parameters, &c, &zeroed);
        assert_relative_eq!(
            output.data.as_flattened(),
            &Matrix {
                data: [[0.125, -0.125], [1.25, -1.25]]
            }
            .data
            .as_flattened(),
            max_relative = 0.01
        );
    }

    #[test]
    fn test_transfer_function_block_matrix_integrator() {
        let c = StubContext::default();
        let num = [1.0];
        let denom = [1.0, -1.0];
        let parameters = super::Parameters::new_arr(&num, &denom);

        let mut block = TransferFunctionBlock::<1, 2, f64, Matrix<2, 2, f64>>::default();

        let input = Matrix {
            data: [[1.0, -1.0], [10.0, -10.0]],
        };
        let output = block.process(&parameters, &c, &input);
        assert_eq!(
            output,
            &Matrix {
                data: [[1.0, -1.0], [10.0, -10.0]]
            }
        );

        let output = block.process(&parameters, &c, &input);
        assert_relative_eq!(
            output.data.as_flattened(),
            &Matrix {
                data: [[2., -2.], [20.0, -20.0]]
            }
            .data
            .as_flattened(),
            max_relative = 0.01
        );

        let zeroed = Matrix::<2, 2, f64>::zeroed();
        let output = block.process(&parameters, &c, &zeroed);
        assert_relative_eq!(
            output.data.as_flattened(),
            &Matrix {
                data: [[2., -2.], [20.0, -20.0]]
            }
            .data
            .as_flattened(),
            max_relative = 0.01
        );

        let input = Matrix {
            data: [[-2.0, 2.0], [-20.0, 20.0]],
        };
        let output = block.process(&parameters, &c, &input);
        assert_relative_eq!(
            output.data.as_flattened(),
            &Matrix {
                data: [[0., 0.], [0., 0.]]
            }
            .data
            .as_flattened(),
            max_relative = 0.01
        );
    }

    #[test]
    fn test_transfer_function_block_matrix_differentiator() {
        let c = StubContext::default();
        let num = [1.0, -1.0];
        let denom = [1.0];
        let parameters = super::Parameters::new_arr(&num, &denom);

        let mut block = TransferFunctionBlock::<2, 1, f64, Matrix<2, 2, f64>>::default();

        let input = Matrix {
            data: [[1.0, -1.0], [5.0, -5.0]],
        };
        let output = block.process(&parameters, &c, &input);
        assert_eq!(
            output,
            &Matrix {
                data: [[1.0, -1.0], [5.0, -5.0]]
            }
        );

        // Same input differentiated is 0
        let output = block.process(&parameters, &c, &input);
        assert_relative_eq!(
            output.data.as_flattened(),
            &Matrix {
                data: [[0., 0.], [0., 0.]]
            }
            .data
            .as_flattened(),
            max_relative = 0.01
        );

        // Go back the other direction
        let zeroed = Matrix::<2, 2, f64>::zeroed();
        let output = block.process(&parameters, &c, &zeroed);
        assert_relative_eq!(
            output.data.as_flattened(),
            &Matrix {
                data: [[-1.0, 1.0], [-5.0, 5.0]]
            }
            .data
            .as_flattened(),
            max_relative = 0.01
        );
    }

    #[test]
    fn test_transfer_function_block_matrix_delay() {
        let c = StubContext::default();
        let num = [0.0, 1.0];
        let denom = [1.0];
        let parameters = super::Parameters::new_arr(&num, &denom);

        let mut block = TransferFunctionBlock::<2, 1, f64, Matrix<2, 2, f64>>::default();

        let input = Matrix {
            data: [[1.0, -1.0], [5.0, -5.0]],
        };
        let output = block.process(&parameters, &c, &input);
        assert_eq!(
            output,
            &Matrix {
                data: [[0., 0.], [0., 0.]]
            }
        );

        let zeroed = Matrix::<2, 2, f64>::zeroed();
        let output = block.process(&parameters, &c, &zeroed);
        assert_relative_eq!(
            output.data.as_flattened(),
            &Matrix {
                data: [[1.0, -1.0], [5.0, -5.0]]
            }
            .data
            .as_flattened(),
            max_relative = 0.01
        );

        let output = block.process(&parameters, &c, &zeroed);
        assert_relative_eq!(
            output.data.as_flattened(),
            &Matrix {
                data: [[0., 0.], [0., 0.]]
            }
            .data
            .as_flattened(),
            max_relative = 0.01
        );
    }
}
