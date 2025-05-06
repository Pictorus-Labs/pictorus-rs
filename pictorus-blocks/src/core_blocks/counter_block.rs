use block_data::{BlockData as OldBlockData, FromPass};
use num_traits::{One, Zero};
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

/// Counter block increments a counter by 1 each iteration for a scalar, vector, or matrix. The
/// counters can be reset using non-zero values of either a single scalar to
/// to reset all counters or a vector/matrix of values that is the same size as the input to
/// reset individual counters.
///
/// If the reset pin is left unconnected, it is assumed to be 0 and the counter will increment
/// on each iteration.
///
/// T = Input type (e.g. `f64` or `Matrix<R, C, f64>`)
/// R = Reset type e.g.  (`bool` or `Matrix<R, C, bool>`)
pub struct CounterBlock<T, R>
where
    R: Pass,
    T: Default + Pass,
    OldBlockData: FromPass<T>,
{
    pub data: OldBlockData,
    phantom: core::marker::PhantomData<R>,
    count: T,
}

impl<T, R> Default for CounterBlock<T, R>
where
    R: Pass,
    T: Default + Pass,
    OldBlockData: FromPass<T>,
{
    fn default() -> Self {
        Self {
            data: <OldBlockData as FromPass<T>>::from_pass(T::default().as_by()),
            count: T::default(),
            phantom: core::marker::PhantomData,
        }
    }
}

macro_rules! counter_impl {
    ($type:ty) => {
        // A reset line for each counter, must be the same size as the counter
        impl ProcessBlock for CounterBlock<$type, bool>
        where
            OldBlockData: FromPass<$type>,
        {
            // The inputs (bool, bool) map to (increment, reset)
            type Inputs = (bool, bool);
            type Output = $type;
            type Parameters = Parameters;

            fn process(
                &mut self,
                _parameters: &Self::Parameters,
                _context: &dyn pictorus_traits::Context,
                _inputs: PassBy<Self::Inputs>,
            ) -> PassBy<Self::Output> {
                // Corelib matrices are indexed array[col][row]
                if _inputs.1 {
                    // Reset if true
                    self.count = <$type>::zero();
                } else if _inputs.0 {
                    // Increment if true
                    self.count += <$type>::one();
                }
                self.data = OldBlockData::from_scalar(self.count.into());
                self.count
            }
        }

        // A single reset line that resets all of the counters to 0
        impl<const ROWS: usize, const COLS: usize> ProcessBlock
            for CounterBlock<Matrix<ROWS, COLS, $type>, bool>
        where
            OldBlockData: FromPass<Matrix<ROWS, COLS, $type>>,
        {
            // The inputs ([bool; N], bool) map to (increment, reset)
            type Inputs = (Matrix<ROWS, COLS, bool>, bool);
            type Output = Matrix<ROWS, COLS, $type>;
            type Parameters = Parameters;

            fn process(
                &mut self,
                _parameters: &Self::Parameters,
                _context: &dyn pictorus_traits::Context,
                _inputs: PassBy<Self::Inputs>,
            ) -> PassBy<Self::Output> {
                // Corelib matrices are indexed array[col][row]
                for i in 0..ROWS {
                    for j in 0..COLS {
                        if _inputs.1 {
                            // Reset if true
                            self.count.data[j][i] = <$type>::zero();
                        } else if _inputs.0.data[j][i] {
                            // Increment if true
                            self.count.data[j][i] += <$type>::one();
                        }
                    }
                }
                self.data = OldBlockData::from_pass(&self.count);
                &self.count
            }
        }

        // A reset line for each counter, must be the same size as the counter
        impl<const ROWS: usize, const COLS: usize> ProcessBlock
            for CounterBlock<Matrix<ROWS, COLS, $type>, Matrix<ROWS, COLS, bool>>
        where
            OldBlockData: FromPass<Matrix<ROWS, COLS, $type>>,
        {
            // The inputs ([bool; N], [bool; N]) map to (increment, reset)
            type Inputs = (Matrix<ROWS, COLS, bool>, Matrix<ROWS, COLS, bool>);
            type Output = Matrix<ROWS, COLS, $type>;
            type Parameters = Parameters;

            fn process(
                &mut self,
                _parameters: &Self::Parameters,
                _context: &dyn pictorus_traits::Context,
                _inputs: PassBy<Self::Inputs>,
            ) -> PassBy<Self::Output> {
                // Corelib matrices are indexed array[col][row]
                for i in 0..ROWS {
                    for j in 0..COLS {
                        if _inputs.1.data[j][i] {
                            // Reset if true
                            self.count.data[j][i] = <$type>::zero();
                        } else if _inputs.0.data[j][i] {
                            self.count.data[j][i] += <$type>::one();
                        }
                    }
                }
                self.data = OldBlockData::from_pass(&self.count);
                &self.count
            }
        }
    };
}

counter_impl!(u8);
counter_impl!(u16);
counter_impl!(u32);
counter_impl!(f32);
counter_impl!(f64);

#[cfg(test)]
mod tests {
    use crate::testing::StubContext;

    use super::*;

    #[test]
    fn test_counter_block_simple_f64() {
        let p = Parameters::new();
        let mut block = CounterBlock::<Matrix<1, 1, f64>, Matrix<1, 1, bool>>::default();
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
        let mut block = CounterBlock::<Matrix<1, 2, f64>, Matrix<1, 2, bool>>::default();
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
        let mut block = CounterBlock::<Matrix<2, 2, f64>, Matrix<2, 2, bool>>::default();
        let c = StubContext::default();

        let mut increment = Matrix::<2, 2, bool>::zeroed();
        increment.data[0][0] = true;
        increment.data[1][0] = true;
        increment.data[0][1] = true;
        increment.data[1][1] = true;

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
        let mut block = CounterBlock::<Matrix<2, 2, f64>, bool>::default();
        let c = StubContext::default();

        let mut increment = Matrix::<2, 2, bool>::zeroed();
        increment.data[0][0] = true;
        increment.data[1][0] = true;
        increment.data[0][1] = true;
        increment.data[1][1] = true;

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
        let mut block = CounterBlock::<Matrix<2, 2, u8>, Matrix<2, 2, bool>>::default();
        let c = StubContext::default();

        let mut increment = Matrix::<2, 2, bool>::zeroed();
        increment.data[0][0] = true;
        increment.data[1][0] = true;
        increment.data[0][1] = true;
        increment.data[1][1] = true;

        let mut reset = Matrix::<2, 2, bool>::zeroed();

        let output = block.process(&p, &c, (&increment, &reset));
        assert_eq!(output.data[0][0], 1);
        assert_eq!(output.data[1][0], 1);
        assert_eq!(output.data[0][1], 1);
        assert_eq!(output.data[1][1], 1);

        let output = block.process(&p, &c, (&increment, &reset));
        assert_eq!(output.data[0][0], 2);
        assert_eq!(output.data[1][0], 2);
        assert_eq!(output.data[0][1], 2);
        assert_eq!(output.data[1][1], 2);

        reset.data[0][0] = true;
        let output = block.process(&p, &c, (&increment, &reset));
        assert_eq!(output.data[0][0], 0);
        assert_eq!(output.data[1][0], 3);
        assert_eq!(output.data[0][1], 3);
        assert_eq!(output.data[1][1], 3);

        reset.data[0][0] = false;
        reset.data[1][0] = true;
        let output = block.process(&p, &c, (&increment, &reset));
        assert_eq!(output.data[0][0], 1);
        assert_eq!(output.data[1][0], 0);
        assert_eq!(output.data[0][1], 4);
        assert_eq!(output.data[1][1], 4);

        reset.data[0][0] = false;
        reset.data[1][0] = false;
        reset.data[0][1] = true;
        let output = block.process(&p, &c, (&increment, &reset));
        assert_eq!(output.data[0][0], 2);
        assert_eq!(output.data[1][0], 1);
        assert_eq!(output.data[0][1], 0);
        assert_eq!(output.data[1][1], 5);
    }

    #[test]
    fn test_counter_block_2x2_u16() {
        let p = Parameters::new();
        let mut block = CounterBlock::<Matrix<2, 2, u16>, Matrix<2, 2, bool>>::default();
        let c = StubContext::default();

        let mut increment = Matrix::<2, 2, bool>::zeroed();
        increment.data[0][0] = true;
        increment.data[1][0] = true;
        increment.data[0][1] = true;
        increment.data[1][1] = true;

        let mut reset = Matrix::<2, 2, bool>::zeroed();

        let output = block.process(&p, &c, (&increment, &reset));
        assert_eq!(output.data[0][0], 1);
        assert_eq!(output.data[1][0], 1);
        assert_eq!(output.data[0][1], 1);
        assert_eq!(output.data[1][1], 1);

        let output = block.process(&p, &c, (&increment, &reset));
        assert_eq!(output.data[0][0], 2);
        assert_eq!(output.data[1][0], 2);
        assert_eq!(output.data[0][1], 2);
        assert_eq!(output.data[1][1], 2);

        reset.data[0][0] = true;
        let output = block.process(&p, &c, (&increment, &reset));
        assert_eq!(output.data[0][0], 0);
        assert_eq!(output.data[1][0], 3);
        assert_eq!(output.data[0][1], 3);
        assert_eq!(output.data[1][1], 3);

        reset.data[0][0] = false;
        reset.data[1][0] = true;
        let output = block.process(&p, &c, (&increment, &reset));
        assert_eq!(output.data[0][0], 1);
        assert_eq!(output.data[1][0], 0);
        assert_eq!(output.data[0][1], 4);
        assert_eq!(output.data[1][1], 4);

        reset.data[0][0] = false;
        reset.data[1][0] = false;
        reset.data[0][1] = true;
        let output = block.process(&p, &c, (&increment, &reset));
        assert_eq!(output.data[0][0], 2);
        assert_eq!(output.data[1][0], 1);
        assert_eq!(output.data[0][1], 0);
        assert_eq!(output.data[1][1], 5);
    }

    #[test]
    fn test_counter_block_2x2_u32() {
        let p = Parameters::new();
        let mut block = CounterBlock::<Matrix<2, 2, u32>, Matrix<2, 2, bool>>::default();
        let c = StubContext::default();

        let mut increment = Matrix::<2, 2, bool>::zeroed();
        increment.data[0][0] = true;
        increment.data[1][0] = true;
        increment.data[0][1] = true;
        increment.data[1][1] = true;

        let mut reset = Matrix::<2, 2, bool>::zeroed();

        let output = block.process(&p, &c, (&increment, &reset));
        assert_eq!(output.data[0][0], 1);
        assert_eq!(output.data[1][0], 1);
        assert_eq!(output.data[0][1], 1);
        assert_eq!(output.data[1][1], 1);

        let output = block.process(&p, &c, (&increment, &reset));
        assert_eq!(output.data[0][0], 2);
        assert_eq!(output.data[1][0], 2);
        assert_eq!(output.data[0][1], 2);
        assert_eq!(output.data[1][1], 2);

        reset.data[0][0] = true;
        let output = block.process(&p, &c, (&increment, &reset));
        assert_eq!(output.data[0][0], 0);
        assert_eq!(output.data[1][0], 3);
        assert_eq!(output.data[0][1], 3);
        assert_eq!(output.data[1][1], 3);

        reset.data[0][0] = false;
        reset.data[1][0] = true;
        let output = block.process(&p, &c, (&increment, &reset));
        assert_eq!(output.data[0][0], 1);
        assert_eq!(output.data[1][0], 0);
        assert_eq!(output.data[0][1], 4);
        assert_eq!(output.data[1][1], 4);

        reset.data[0][0] = false;
        reset.data[1][0] = false;
        reset.data[0][1] = true;
        let output = block.process(&p, &c, (&increment, &reset));
        assert_eq!(output.data[0][0], 2);
        assert_eq!(output.data[1][0], 1);
        assert_eq!(output.data[0][1], 0);
        assert_eq!(output.data[1][1], 5);
    }
}
