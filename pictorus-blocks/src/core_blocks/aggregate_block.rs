use crate::traits::Scalar;
use num_traits::AsPrimitive;
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock};

/// Block for performing an aggregation operation (i.e. sum, min, max) on input data.
pub struct AggregateBlock<T: Apply> {
    buffer: T::Output,
}

impl<T: Apply> Default for AggregateBlock<T>
where
    T: Pass + Default,
{
    fn default() -> Self {
        Self {
            buffer: <T::Output>::default(),
        }
    }
}

impl<T> ProcessBlock for AggregateBlock<T>
where
    T: Apply + Default,
{
    type Inputs = T;
    type Output = T::Output;
    type Parameters = Parameters;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) -> pictorus_traits::PassBy<'b, Self::Output> {
        let output = T::apply(&mut self.buffer, inputs, parameters.method);
        output
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        self.buffer.as_by()
    }
}

pub trait Apply: Pass {
    type Output: Scalar;

    fn apply<'s>(
        store: &mut Self::Output,
        input: PassBy<Self>,
        method: AggregateMethod,
    ) -> PassBy<'s, Self::Output>;
}

impl<T: Scalar> Apply for T {
    type Output = T;

    fn apply<'s>(
        store: &mut Self::Output,
        input: PassBy<Self>,
        _method: AggregateMethod,
    ) -> PassBy<'s, Self::Output> {
        *store = input;
        input
    }
}

impl<const NROWS: usize, const NCOLS: usize, T: Scalar + num_traits::Num + num_traits::Bounded>
    Apply for Matrix<NROWS, NCOLS, T>
where
    usize: AsPrimitive<T>,
    Matrix<NROWS, NCOLS, T>: Copy,
{
    type Output = T;

    fn apply<'s>(
        store: &mut Self::Output,
        input: PassBy<Self>,
        method: AggregateMethod,
    ) -> PassBy<'s, Self::Output> {
        let elems = input.data.as_flattened();
        let elems_len = elems.len();
        *store = match method {
            AggregateMethod::Sum => elems.iter().fold(T::default(), |acc, x| acc + *x),
            AggregateMethod::Mean => {
                elems.iter().fold(T::default(), |acc, x| acc + *x) / elems_len.as_()
            }
            AggregateMethod::Max => {
                elems
                    .iter()
                    .fold(T::min_value(), |acc, x| if *x > acc { *x } else { acc })
            }
            AggregateMethod::Min => {
                elems
                    .iter()
                    .fold(T::max_value(), |acc, x| if *x < acc { *x } else { acc })
            }
            AggregateMethod::Median => {
                let mut sorted = *input;
                let sorted = sorted.data.as_flattened_mut();
                sorted.sort_unstable_by(|a, b| {
                    a.partial_cmp(b).expect("NaNs and INFs are not supported")
                });
                let mid = sorted.len() / 2;
                if sorted.len() % 2 == 0 {
                    (sorted[mid - 1] + sorted[mid]) / (T::one() + T::one())
                } else {
                    sorted[mid]
                }
            }
        };
        *store
    }
}

/// Represents the method of aggregation to be performed.
#[derive(Debug, Clone, Copy, PartialEq, strum::EnumString)]
pub enum AggregateMethod {
    /// Sum of all elements.
    Sum,
    /// Mean (average) of all elements.
    Mean,
    /// Median of all elements.
    Median,
    /// Minimum value among all elements.
    Min,
    /// Maximum value among all elements.
    Max,
}

pub struct Parameters {
    pub method: AggregateMethod,
}
impl Parameters {
    pub fn new(method: &str) -> Self {
        Self {
            method: method.parse().expect("Invalid aggregate method"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use alloc::str::FromStr;
    use approx::assert_relative_eq;

    const PARAM_MEDIAN: Parameters = Parameters {
        method: AggregateMethod::Median,
    };
    const PARAM_MIN: Parameters = Parameters {
        method: AggregateMethod::Min,
    };
    const PARAM_MAX: Parameters = Parameters {
        method: AggregateMethod::Max,
    };
    const PARAM_SUM: Parameters = Parameters {
        method: AggregateMethod::Sum,
    };
    const PARAM_MEAN: Parameters = Parameters {
        method: AggregateMethod::Mean,
    };

    #[test]
    fn test_aggregate_default_buffer_no_panic() {
        let block = AggregateBlock::<Matrix<4, 7, f64>>::default();
        assert_eq!(block.buffer(), 0.0);
    }

    #[test]
    fn test_aggregate_sum_f32() {
        let mut block = AggregateBlock::<Matrix<4, 7, f32>>::default();
        let context = StubContext::default();
        let input: Matrix<4, 7, f32> = Matrix {
            data: [[1.0; 4]; 7],
        };
        let output = block.process(&PARAM_SUM, &context, &input);
        assert_relative_eq!(output, 28.0);
        assert_relative_eq!(block.buffer(), output);
    }

    #[test]
    fn test_aggregate_sum_f64() {
        let mut block = AggregateBlock::<Matrix<4, 7, f64>>::default();
        let context = StubContext::default();
        let input: Matrix<4, 7, f64> = Matrix {
            data: [[1.0; 4]; 7],
        };
        let output = block.process(&PARAM_SUM, &context, &input);
        assert_relative_eq!(output, 28.0);
        assert_relative_eq!(block.buffer(), output);
    }

    #[test]
    fn test_aggregate_max_f64() {
        let mut block = AggregateBlock::<Matrix<4, 7, f64>>::default();
        let context = StubContext::default();
        let mut input: Matrix<4, 7, f64> = Matrix {
            data: [[1.0; 4]; 7],
        };
        input.data[5][3] = 42.0;
        let output = block.process(&PARAM_MAX, &context, &input);
        assert_relative_eq!(output, 42.0);
        assert_relative_eq!(block.buffer(), output);
    }

    #[test]
    fn test_aggregate_min_f64() {
        let mut block = AggregateBlock::<Matrix<4, 7, f64>>::default();
        let context = StubContext::default();
        let mut input: Matrix<4, 7, f64> = Matrix {
            data: [[11.0; 4]; 7],
        };
        input.data[1][2] = 10.99;
        let output = block.process(&PARAM_MIN, &context, &input);
        assert_relative_eq!(output, 10.99);
        assert_relative_eq!(block.buffer(), output);
    }

    #[test]
    fn test_aggregate_mean_f64() {
        let mut block = AggregateBlock::<Matrix<4, 7, f64>>::default();
        let context = StubContext::default();
        let mut input: Matrix<4, 7, f64> = Matrix::zeroed();
        for (idx, elem) in input.data.as_flattened_mut().iter_mut().enumerate() {
            *elem = idx as f64;
        }

        let output = block.process(&PARAM_MEAN, &context, &input);
        assert_relative_eq!(output, 13.5);
        assert_relative_eq!(block.buffer(), output);
    }

    #[test]
    fn test_aggregate_median_f64() {
        let mut block = AggregateBlock::<Matrix<4, 7, f64>>::default();
        let context = StubContext::default();
        let mut input: Matrix<4, 7, f64> = Matrix::zeroed();
        for (idx, elem) in input.data.as_flattened_mut().iter_mut().enumerate() {
            *elem = idx as f64;
        }

        let output = block.process(&PARAM_MEDIAN, &context, &input);
        assert_relative_eq!(output, 13.5);
        assert_relative_eq!(block.buffer(), output);
    }

    #[test]
    fn test_smattering_of_types() {
        let context = StubContext::default();

        let mut block = AggregateBlock::<Matrix<2, 2, u8>>::default();
        let input = Matrix {
            data: [[1, 2], [3, 4]],
        };
        let output = block.process(&PARAM_MEDIAN, &context, &input);
        assert_eq!(output, 2);
        let output = block.process(&PARAM_MEAN, &context, &input);
        assert_eq!(output, 2);

        let mut block = AggregateBlock::<Matrix<2, 2, i32>>::default();
        let input = Matrix {
            data: [[-34, 200], [31, 4]],
        };
        let output = block.process(&PARAM_MIN, &context, &input);
        assert_eq!(output, -34);

        let mut block = AggregateBlock::<Matrix<2, 2, i8>>::default();
        let input = Matrix {
            data: [[-34, 127], [31, 4]],
        };
        let output = block.process(&PARAM_MAX, &context, &input);
        assert_eq!(output, 127);

        let mut block = AggregateBlock::<Matrix<2, 2, u32>>::default();
        let input = Matrix {
            data: [[34, 127], [31, 4]],
        };
        let output = block.process(&PARAM_SUM, &context, &input);
        assert_eq!(output, 196);
    }

    #[test]
    #[should_panic]
    fn overflow_in_add_panics() {
        let mut block = AggregateBlock::<Matrix<2, 2, u8>>::default();
        let input = Matrix {
            data: [[34, 127], [128, 4]],
        };
        let context = StubContext::default();
        let output = block.process(&PARAM_SUM, &context, &input);
        assert_eq!(output, 196);
    }

    #[test]
    #[should_panic]
    fn overflow_in_mean_panics() {
        let mut block = AggregateBlock::<Matrix<2, 2, u8>>::default();
        let input = Matrix {
            data: [[34, 127], [128, 4]],
        };
        let context = StubContext::default();
        let output = block.process(&PARAM_MEAN, &context, &input);
        assert_eq!(output, 196);
    }

    #[test]
    fn test_aggregate_method_from_str() {
        assert_eq!(
            AggregateMethod::from_str("Sum").unwrap(),
            AggregateMethod::Sum
        );
        assert_eq!(
            AggregateMethod::from_str("Mean").unwrap(),
            AggregateMethod::Mean
        );
        assert_eq!(
            AggregateMethod::from_str("Median").unwrap(),
            AggregateMethod::Median
        );
        assert_eq!(
            AggregateMethod::from_str("Min").unwrap(),
            AggregateMethod::Min
        );
        assert_eq!(
            AggregateMethod::from_str("Max").unwrap(),
            AggregateMethod::Max
        );
        assert!(AggregateMethod::from_str("Invalid").is_err());
    }
}
