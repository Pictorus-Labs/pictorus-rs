use block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock};

use crate::traits::Scalar;

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

/// Increments a counter every time the count input is truthy.
///
/// The counters can be reset using non-zero values of either a single scalar to
/// to reset all counters or a vector/matrix of values that is the same size as the input to
/// reset individual counters.
///
/// The block is generic over a type 'T'. This is expected to be a tuple of two types, the first
/// is the input type and the second is the reset type. For both types they accepts either a scalar
/// or a matrix of scalars. However they are interpreted as bools or matrices of bools, where true or
/// false is determined by whether the value is non-zero or zero respectively. See the [`Scalar::is_truthy`]
/// function for more details.
pub struct CounterBlock<T: Apply>
where
    OldBlockData: FromPass<T::Counter>,
{
    pub data: OldBlockData,
    counter: T::Counter,
}

impl<T: Apply> ProcessBlock for CounterBlock<T>
where
    OldBlockData: FromPass<T::Counter>,
{
    type Inputs = T;
    type Output = T::Counter;
    type Parameters = Parameters;

    fn process<'b>(
        &'b mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        T::apply(&mut self.counter, inputs)
    }
}

impl<T: Apply> Default for CounterBlock<T>
where
    OldBlockData: FromPass<T::Counter>,
{
    fn default() -> Self {
        let counter = T::Counter::default();
        Self {
            data: OldBlockData::from_pass(counter.as_by()),
            counter,
        }
    }
}

pub trait Apply: Pass {
    type Counter: Default + Pass;
    fn apply<'a>(count: &'a mut Self::Counter, input: PassBy<Self>) -> PassBy<'a, Self::Counter>;
}

impl<I: Scalar, R: Scalar> Apply for (I, R) {
    type Counter = f64;
    fn apply<'a>(count: &'a mut Self::Counter, input: PassBy<Self>) -> PassBy<'a, Self::Counter> {
        if input.1.is_truthy() {
            *count = 0.0;
        } else if input.0.is_truthy() {
            *count += 1.0;
        }
        count.as_by()
    }
}

impl<I: Scalar, R: Scalar, const NROWS: usize, const NCOLS: usize> Apply
    for (Matrix<NROWS, NCOLS, I>, R)
{
    type Counter = Matrix<NROWS, NCOLS, f64>;
    fn apply<'a>(count: &'a mut Self::Counter, input: PassBy<Self>) -> PassBy<'a, Self::Counter> {
        for i in 0..NROWS {
            for j in 0..NCOLS {
                if input.1.is_truthy() {
                    count.data[j][i] = 0.0;
                } else if input.0.data[j][i].is_truthy() {
                    count.data[j][i] += 1.0;
                }
            }
        }
        count.as_by()
    }
}

impl<I: Scalar, R: Scalar, const NROWS: usize, const NCOLS: usize> Apply
    for (Matrix<NROWS, NCOLS, I>, Matrix<NROWS, NCOLS, R>)
{
    type Counter = Matrix<NROWS, NCOLS, f64>;
    fn apply<'a>(count: &'a mut Self::Counter, input: PassBy<Self>) -> PassBy<'a, Self::Counter> {
        for i in 0..NROWS {
            for j in 0..NCOLS {
                if input.1.data[j][i].is_truthy() {
                    count.data[j][i] = 0.0;
                } else if input.0.data[j][i].is_truthy() {
                    count.data[j][i] += 1.0;
                }
            }
        }
        count.as_by()
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::StubContext;

    use super::*;

    #[test]
    fn test_counter_block_simple_f64() {
        let p = Parameters::new();
        let mut block = CounterBlock::<(Matrix<1, 1, bool>, Matrix<1, 1, bool>)>::default();
        let c = StubContext::default();

        let mut increment = Matrix::<1, 1, bool>::zeroed();
        increment.data[0][0] = true;

        let mut reset = Matrix::<1, 1, bool>::zeroed();
        reset.data[0][0] = false;

        let output = block.process(&p, &c, (&increment, &reset));
        assert!(output.data[0][0] == 1.0);

        let output = block.process(&p, &c, (&increment, &reset));
        assert!(output.data[0][0] == 2.0);

        reset.data[0][0] = true;
        let output = block.process(&p, &c, (&increment, &reset));
        assert!(output.data[0][0] == 0.0);
    }

    #[test]
    fn test_counter_block_1x2_f64() {
        let p = Parameters::new();
        let mut block = CounterBlock::<(Matrix<1, 2, bool>, Matrix<1, 2, bool>)>::default();
        let c = StubContext::default();

        let mut increment = Matrix::<1, 2, bool>::zeroed();
        increment.data[0][0] = true;

        let mut reset = Matrix::<1, 2, bool>::zeroed();
        reset.data[0][0] = false;

        let output = block.process(&p, &c, (&increment, &reset));
        assert_eq!(output.data[0][0], 1.0);
        assert_eq!(output.data[1][0], 0.0);

        let output = block.process(&p, &c, (&increment, &reset));
        assert_eq!(output.data[0][0], 2.0);
        assert_eq!(output.data[1][0], 0.0);

        reset.data[0][0] = true;
        let output = block.process(&p, &c, (&increment, &reset));
        assert_eq!(output.data[0][0], 0.0);
        assert_eq!(output.data[1][0], 0.0);
    }

    #[test]
    fn test_counter_block_2x2_f64() {
        let p = Parameters::new();
        let mut block = CounterBlock::<(Matrix<2, 2, f64>, Matrix<2, 2, bool>)>::default();
        let c = StubContext::default();

        let mut increment = Matrix::<2, 2, f64>::zeroed();
        increment.data[0][0] = 1.0;
        increment.data[1][0] = 1.0;
        increment.data[0][1] = 1.0;
        increment.data[1][1] = 1.0;

        let mut reset = Matrix::<2, 2, bool>::zeroed();

        let output = block.process(&p, &c, (&increment, &reset));
        assert_eq!(output.data[0][0], 1.0);
        assert_eq!(output.data[1][0], 1.0);
        assert_eq!(output.data[0][1], 1.0);
        assert_eq!(output.data[1][1], 1.0);

        let output = block.process(&p, &c, (&increment, &reset));
        assert_eq!(output.data[0][0], 2.0);
        assert_eq!(output.data[1][0], 2.0);
        assert_eq!(output.data[0][1], 2.0);
        assert_eq!(output.data[1][1], 2.0);

        reset.data[0][0] = true;
        let output = block.process(&p, &c, (&increment, &reset));
        assert_eq!(output.data[0][0], 0.0);
        assert_eq!(output.data[1][0], 3.0);
        assert_eq!(output.data[0][1], 3.0);
        assert_eq!(output.data[1][1], 3.0);

        reset.data[0][0] = false;
        reset.data[1][0] = true;
        let output = block.process(&p, &c, (&increment, &reset));
        assert_eq!(output.data[0][0], 1.0);
        assert_eq!(output.data[1][0], 0.0);
        assert_eq!(output.data[0][1], 4.0);
        assert_eq!(output.data[1][1], 4.0);

        reset.data[0][0] = false;
        reset.data[1][0] = false;
        reset.data[0][1] = true;
        let output = block.process(&p, &c, (&increment, &reset));
        assert_eq!(output.data[0][0], 2.0);
        assert_eq!(output.data[1][0], 1.0);
        assert_eq!(output.data[0][1], 0.0);
        assert_eq!(output.data[1][1], 5.0);
    }

    #[test]
    fn test_counter_block_2x2_single_reset_f64() {
        let p = Parameters::new();
        let mut block = CounterBlock::<(Matrix<2, 2, f64>, bool)>::default();
        let c = StubContext::default();

        let mut increment = Matrix::<2, 2, f64>::zeroed();
        increment.data[0][0] = 1.0;
        increment.data[1][0] = 1.0;
        increment.data[0][1] = 1.0;
        increment.data[1][1] = 1.0;

        let mut reset = false;

        let output = block.process(&p, &c, (&increment, reset));
        assert_eq!(output.data[0][0], 1.0);
        assert_eq!(output.data[1][0], 1.0);
        assert_eq!(output.data[0][1], 1.0);
        assert_eq!(output.data[1][1], 1.0);

        let output = block.process(&p, &c, (&increment, reset));
        assert_eq!(output.data[0][0], 2.0);
        assert_eq!(output.data[1][0], 2.0);
        assert_eq!(output.data[0][1], 2.0);
        assert_eq!(output.data[1][1], 2.0);

        reset = true;
        let output = block.process(&p, &c, (&increment, reset));
        assert_eq!(output.data[0][0], 0.0);
        assert_eq!(output.data[1][0], 0.0);
        assert_eq!(output.data[0][1], 0.0);
        assert_eq!(output.data[1][1], 0.0);

        reset = false;
        let output = block.process(&p, &c, (&increment, reset));
        assert_eq!(output.data[0][0], 1.0);
        assert_eq!(output.data[1][0], 1.0);
        assert_eq!(output.data[0][1], 1.0);
        assert_eq!(output.data[1][1], 1.0);

        let output = block.process(&p, &c, (&increment, reset));
        assert_eq!(output.data[0][0], 2.0);
        assert_eq!(output.data[1][0], 2.0);
        assert_eq!(output.data[0][1], 2.0);
        assert_eq!(output.data[1][1], 2.0);
    }

    #[test]
    fn test_counter_block_2x2_u8() {
        let p = Parameters::new();
        let mut block = CounterBlock::<(Matrix<2, 2, u8>, Matrix<2, 2, bool>)>::default();
        let c = StubContext::default();

        let mut increment = Matrix::<2, 2, u8>::zeroed();
        increment.data[0][0] = 1;
        increment.data[1][0] = 1;
        increment.data[0][1] = 1;
        increment.data[1][1] = 1;

        let mut reset = Matrix::<2, 2, bool>::zeroed();

        let output = block.process(&p, &c, (&increment, &reset));
        assert_eq!(output.data[0][0], 1.0);
        assert_eq!(output.data[1][0], 1.0);
        assert_eq!(output.data[0][1], 1.0);
        assert_eq!(output.data[1][1], 1.0);

        let output = block.process(&p, &c, (&increment, &reset));
        assert_eq!(output.data[0][0], 2.0);
        assert_eq!(output.data[1][0], 2.0);
        assert_eq!(output.data[0][1], 2.0);
        assert_eq!(output.data[1][1], 2.0);

        reset.data[0][0] = true;
        let output = block.process(&p, &c, (&increment, &reset));
        assert_eq!(output.data[0][0], 0.0);
        assert_eq!(output.data[1][0], 3.0);
        assert_eq!(output.data[0][1], 3.0);
        assert_eq!(output.data[1][1], 3.0);

        reset.data[0][0] = false;
        reset.data[1][0] = true;
        let output = block.process(&p, &c, (&increment, &reset));
        assert_eq!(output.data[0][0], 1.0);
        assert_eq!(output.data[1][0], 0.0);
        assert_eq!(output.data[0][1], 4.0);
        assert_eq!(output.data[1][1], 4.0);

        reset.data[0][0] = false;
        reset.data[1][0] = false;
        reset.data[0][1] = true;
        let output = block.process(&p, &c, (&increment, &reset));
        assert_eq!(output.data[0][0], 2.0);
        assert_eq!(output.data[1][0], 1.0);
        assert_eq!(output.data[0][1], 0.0);
        assert_eq!(output.data[1][1], 5.0);
    }
}
