use crate::traits::Scalar;
use block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock};

/// Raises the input to a specified power (coefficient),
/// and optionally preserves the sign of the input.
///
/// It can accept a scalar or a matrix input. If the input is a matrix,
/// the exponentiation is performed element-wise.
///
/// The power to raise the input to as well as a flag to optionally preserve the sign
/// of the input when performing the exponentiation can be set in the parameters.
///
/// # Panics
/// If the input is negative and the coefficient is < 1.0 and preserve_sign is false,
/// a panic will occur.
#[derive(Debug)]
pub struct ExponentBlock<T: Pass + Default> {
    pub data: OldBlockData,
    output: Option<T>,
}

impl<T: Pass + Default> Default for ExponentBlock<T>
where
    OldBlockData: FromPass<T>,
{
    fn default() -> Self {
        Self {
            data: <OldBlockData as FromPass<T>>::from_pass(T::default().as_by()),
            output: None,
        }
    }
}

impl<S: Scalar + num_traits::Float + num_traits::Zero> ProcessBlock for ExponentBlock<S>
where
    OldBlockData: FromPass<S>,
{
    type Inputs = S;
    type Output = S;
    type Parameters = Parameters<S>;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        let mut inputs_local = inputs;
        if (inputs < S::zero()) && (parameters.coefficient < S::one()) {
            if !parameters.preserve_sign {
                panic!("Negative input to Exponent with coefficient < 1.0!");
            } else {
                inputs_local = inputs_local.abs();
            }
        }
        let output = self
            .output
            .insert(inputs_local.powf(parameters.coefficient));
        if parameters.preserve_sign {
            let should_flip_sign = (*output < S::zero()) != (inputs < S::zero());
            if should_flip_sign {
                *output = output.neg();
            };
        }
        self.data = OldBlockData::from_pass(*output);
        *output
    }
}

impl<S: Scalar + num_traits::Float + num_traits::Zero, const NROWS: usize, const NCOLS: usize>
    ProcessBlock for ExponentBlock<Matrix<NROWS, NCOLS, S>>
where
    OldBlockData: FromPass<Matrix<NROWS, NCOLS, S>>,
{
    type Inputs = Matrix<NROWS, NCOLS, S>;
    type Output = Matrix<NROWS, NCOLS, S>;
    type Parameters = Parameters<S>;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        let output = self.output.insert(*inputs);
        output.data.as_flattened_mut().iter_mut().for_each(|x| {
            let mut x_local = *x;
            if (x_local < S::zero()) && (parameters.coefficient < S::one()) {
                if !parameters.preserve_sign {
                    panic!("Negative input to Exponent with coefficient < 1.0!");
                } else {
                    x_local = x_local.abs();
                }
            }
            x_local = x_local.powf(parameters.coefficient);
            if parameters.preserve_sign {
                let should_flip_sign = (x_local < S::zero()) != (*x < S::zero());
                if should_flip_sign {
                    x_local = x_local.neg();
                };
            }
            *x = x_local;
        });
        self.data = OldBlockData::from_pass(output);
        output
    }
}

/// Parameters for the ExponentBlock
#[derive(Debug, Clone, Copy)]
pub struct Parameters<T: Scalar + num_traits::Float> {
    /// The coefficient to raise the input to
    /// has the effect of being a root if < 1.0
    coefficient: T,
    /// Whether to preserve the sign of the input
    /// when performing the exponentiation.
    /// If the [`coefficient`] is < 1.0 and the input is negative,
    /// this will cause a panic if set to false.
    preserve_sign: bool,
}

impl<T: Scalar + num_traits::Float> Parameters<T> {
    pub fn new<S: Scalar>(coefficient: T, preserve_sign: S) -> Self {
        Self {
            coefficient,
            preserve_sign: preserve_sign.is_truthy(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;

    #[test]
    fn test_exponent_block_scalar() {
        let context = StubContext::default();
        let mut block = ExponentBlock::<f64>::default();

        // Preserve sign is false
        let parameters = Parameters::new(2.0, false);
        let input = 2.0;
        let output = block.process(&parameters, &context, input.as_by());
        assert_eq!(output, 4.0);
        let input = -2.0;
        let output = block.process(&parameters, &context, input.as_by());
        assert_eq!(output, 4.0);

        // Preserve sign is true
        let parameters = Parameters::new(4.0, true);
        let input = 11.0;
        let output = block.process(&parameters, &context, input.as_by());
        assert_eq!(output, 14641.0);
        let input = -11.0;
        let output = block.process(&parameters, &context, input.as_by());
        assert_eq!(output, -14641.0);

        // Now try a Root
        let parameters = Parameters::new(0.5, false);
        let input = 4.0;
        let output = block.process(&parameters, &context, input.as_by());
        assert_eq!(output, 2.0);

        // Now try a Root with preserve sign
        let parameters = Parameters::new(0.5, true);
        let output = block.process(&parameters, &context, input.as_by());
        assert_eq!(output, 2.0);
        let input = -4.0;
        let output = block.process(&parameters, &context, input.as_by());
        assert_eq!(output, -2.0);
    }

    #[test]
    #[should_panic]
    fn test_root_negative_input_no_preserve_sign_panic() {
        let context = StubContext::default();
        let mut block = ExponentBlock::<f64>::default();
        let parameters = Parameters::new(0.5, false);
        let input = -4.0;
        block.process(&parameters, &context, input.as_by());
    }

    #[test]
    fn test_exponent_block_matrix() {
        let context = StubContext::default();
        let mut block = ExponentBlock::<Matrix<2, 2, f32>>::default();

        // Preserve sign is false
        let parameters = Parameters::new(2.0, false);
        let input = Matrix {
            data: [[1.0, -2.0], [3.0, -4.0]],
        };
        let output = block.process(&parameters, &context, &input);
        assert_eq!(output.data, [[1.0, 4.0], [9.0, 16.0]]);

        // Preserve sign is true
        let parameters = Parameters::new(4.0, true);
        let output = block.process(&parameters, &context, &input);
        assert_eq!(output.data, [[1.0, -16.0], [81.0, -256.0]]);

        // Now try a Root
        let parameters = Parameters::new(0.5, false);
        let input = Matrix {
            data: [[1.0, 4.0], [9.0, 16.0]],
        };
        let output = block.process(&parameters, &context, &input);
        assert_eq!(output.data, [[1.0, 2.0], [3.0, 4.0]]);

        // Now try a Root with preserve sign
        let parameters = Parameters::new(0.5, true);
        let output = block.process(&parameters, &context, &input);
        assert_eq!(output.data, [[1.0, 2.0], [3.0, 4.0]]);

        let input = Matrix {
            data: [[1.0, -4.0], [9.0, -16.0]],
        };
        let output = block.process(&parameters, &context, &input);
        assert_eq!(output.data, [[1.0, -2.0], [3.0, -4.0]]);
    }

    #[test]
    #[should_panic]
    fn test_root_matrix_negative_input_no_preserve_sign_panic() {
        let context = StubContext::default();
        let mut block = ExponentBlock::<Matrix<2, 2, f32>>::default();
        let parameters = Parameters::new(0.5, false);
        let input = Matrix {
            data: [[1.0, -4.0], [9.0, -16.0]],
        };
        block.process(&parameters, &context, &input);
    }
}
