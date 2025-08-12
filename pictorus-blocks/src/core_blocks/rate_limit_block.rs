use crate::traits::{Float, MatrixOps};
use num_traits::Zero;
use pictorus_block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock, Scalar};

/// Rate limit block parameters
pub struct Parameters<S: Scalar> {
    /// The maximum rate per second at which the value can increase
    pub rising_rate: S,
    /// The maximum rate per second at which the value can decrease
    pub falling_rate: S,
}

impl<S> Parameters<S>
where
    S: Scalar + Zero,
{
    pub fn new(rising_rate: S, falling_rate: S) -> Self {
        Self {
            rising_rate,
            falling_rate,
        }
    }
}

/// Emits the input signal, but constraining
/// the rate of change of the signal as specified by the Rising and
/// Falling rates.
pub struct RateLimitBlock<T> {
    pub data: OldBlockData,
    buffer: T,
}

impl<T> Default for RateLimitBlock<T>
where
    T: Pass + Default,
    OldBlockData: FromPass<T>,
{
    fn default() -> Self {
        Self {
            data: <OldBlockData as FromPass<T>>::from_pass(T::default().as_by()),
            buffer: T::default(),
        }
    }
}

macro_rules! impl_rate_limit_block {
    ($type:ty) => {
        impl ProcessBlock for RateLimitBlock<$type>
        where
            $type: num_traits::Zero,
            OldBlockData: FromPass<$type>,
        {
            type Inputs = $type;
            type Output = $type;
            type Parameters = Parameters<$type>;

            fn process(
                &mut self,
                parameters: &Self::Parameters,
                context: &dyn pictorus_traits::Context,
                input: PassBy<Self::Inputs>,
            ) -> PassBy<'_, Self::Output> {
                if let Some(timestep_duration) = context.timestep() {
                    let timestep_s = <$type>::from_duration(timestep_duration);
                    let change_rate = (input - self.buffer) / timestep_s;
                    let clamped_change_rate =
                        change_rate.clamp(parameters.falling_rate, parameters.rising_rate);

                    self.buffer = if change_rate.is_nan() {
                        // This can happen if the timestep is zero and `input - self.buffer` == 0)
                        self.buffer
                    } else {
                        // b + clamped_change_rate * timestep_s;
                        self.buffer + clamped_change_rate * timestep_s
                    };
                    self.data = OldBlockData::from_scalar(self.buffer.into());
                    self.buffer
                } else {
                    //First Run ever
                    self.buffer
                }
            }
        }

        impl<const ROWS: usize, const COLS: usize> ProcessBlock
            for RateLimitBlock<Matrix<ROWS, COLS, $type>>
        where
            OldBlockData: FromPass<Matrix<ROWS, COLS, $type>>,
        {
            type Inputs = Matrix<ROWS, COLS, $type>;
            type Output = Matrix<ROWS, COLS, $type>;
            type Parameters = Parameters<$type>;

            fn process(
                &mut self,
                parameters: &Self::Parameters,
                context: &dyn pictorus_traits::Context,
                input: PassBy<Self::Inputs>,
            ) -> PassBy<'_, Self::Output> {
                if let Some(timestep_duration) = context.timestep() {
                    let timestep_s = <$type>::from_duration(timestep_duration);
                    let mut output = Matrix::zeroed();
                    input.for_each(|v, c, r| {
                        let change_rate = (v - self.buffer.data[c][r]) / timestep_s;
                        let clamped_change_rate =
                            change_rate.clamp(parameters.falling_rate, parameters.rising_rate);
                        output.data[c][r] = if change_rate.is_nan() {
                            // This can happen if the timestep is zero and `v - self.buffer.data[c][r]` == 0)
                            self.buffer.data[c][r]
                        } else {
                            self.buffer.data[c][r] + clamped_change_rate * timestep_s
                        }
                    });

                    self.buffer = output;
                    self.data = OldBlockData::from_pass(&output);
                    &self.buffer
                } else {
                    //First Run ever
                    &self.buffer
                }
            }
        }
    };
}

impl_rate_limit_block!(f32);
impl_rate_limit_block!(f64);

#[cfg(test)]
mod tests {
    use core::time;

    use super::*;
    use crate::testing::StubRuntime;
    use core::time::Duration;
    use paste::paste;
    use pictorus_block_data::BlockData as OldBlockData;

    macro_rules! impl_rate_limit_test {
        ($type:ty) => {
            paste! {
                #[test]
                fn [<test_rate_limit_block_scalar_ $type>]() {
                    let rising_rate: f64 = 2.0;
                    let falling_rate: f64 = -1.0;
                    let mut block = RateLimitBlock::<f64>::default();
                    let parameters = Parameters {
                        rising_rate,
                        falling_rate,
                    };
                    let timestep_s = 1.0;

                    let mut runtime = StubRuntime::default();
                    runtime.context.fundamental_timestep = time::Duration::from_secs_f64(timestep_s);

                    // Test rising rate
                    runtime.tick();
                    let output = block.process(&parameters, &runtime.context(), 3.0);
                    assert_eq!(block.data.scalar(), 2.0);
                    assert_eq!(output, 2.0);

                    // Test rising rate
                    runtime.tick();
                    let output = block.process(&parameters, &runtime.context(), 30.0);
                    assert_eq!(block.data.scalar(), 4.0);
                    assert_eq!(output, 4.0);

                    // Value doesn't change if input matches current state
                    runtime.tick();
                    let output = block.process(&parameters, &runtime.context(), 4.0);
                    assert_eq!(block.data.scalar(), 4.0);
                    assert_eq!(output, 4.0);

                    // Test falling rate
                    runtime.tick();
                    let output = block.process(&parameters, &runtime.context(), -30.0);
                    assert_eq!(block.data.scalar(), 3.0);
                    assert_eq!(output, 3.0);

                    // Test falling rate
                    runtime.tick();
                    let output = block.process(&parameters, &runtime.context(), -0.5);
                    assert_eq!(block.data.scalar(), 2.0);
                    assert_eq!(output, 2.0);

                    // Test passing in no timestep does not change output
                    runtime.context.timestep = Some(Duration::from_secs(0));
                    let output = block.process(&parameters, &runtime.context(), -30.0);
                    assert_eq!(block.data.scalar(), 2.0);
                    assert_eq!(output, 2.0);
                }

                #[test]
                fn [<test_rate_limit_block_matrix_ $type>]() {
                    let rising_rate: f64 = 2.0;
                    let falling_rate: f64 = -1.0;
                    let mut block = RateLimitBlock::<Matrix<2, 2, f64>>::default();
                    let parameters = Parameters {
                        rising_rate,
                        falling_rate,
                    };
                    let timestep_s = 1.0;

                    let mut runtime = StubRuntime::default();
                    runtime.context.fundamental_timestep = time::Duration::from_secs_f64(timestep_s);

                    // Test rising rate
                    runtime.tick();
                    let inputs = Matrix {
                        data: [[3.0, 5.0], [6.0, 8.0]],
                    };
                    let output = block.process(&parameters, &runtime.context(), &inputs);
                    assert_eq!(
                        output,
                        &Matrix {
                            data: [[2.0, 2.0], [2.0, 2.0]],
                        }
                    );
                    assert_eq!(
                        block.data,
                        OldBlockData::from_matrix(&[&[2.0, 2.0], &[2.0, 2.0]])
                    );

                    // Test rising rate
                    runtime.tick();
                    let inputs = Matrix {
                        data: [[30.0, 5.0], [6.0, 8.0]],
                    };
                    let output = block.process(&parameters, &runtime.context(), &inputs);
                    assert_eq!(
                        output,
                        &Matrix {
                            data: [[4.0, 4.0], [4.0, 4.0]],
                        }
                    );
                    assert_eq!(
                        block.data,
                        OldBlockData::from_matrix(&[&[4.0, 4.0], &[4.0, 4.0]])
                    );

                    // Value doesn't change if input matches current state
                    runtime.tick();
                    let inputs = Matrix {
                        data: [[4.0, 4.0], [4.0, 4.0]],
                    };
                    let output = block.process(&parameters, &runtime.context(), &inputs);
                    assert_eq!(
                        output,
                        &Matrix {
                            data: [[4.0, 4.0], [4.0, 4.0]],
                        }
                    );
                    assert_eq!(
                        block.data,
                        OldBlockData::from_matrix(&[&[4.0, 4.0], &[4.0, 4.0]])
                    );

                    // Test falling rate
                    runtime.tick();
                    let inputs = Matrix {
                        data: [[-30.0, -2.0], [3.0, 3.8]],
                    };
                    let output = block.process(&parameters, &runtime.context(), &inputs);
                    assert_eq!(
                        output,
                        &Matrix {
                            data: [[3.0, 3.0], [3.0, 3.8]],
                        }
                    );
                    assert_eq!(
                        block.data,
                        OldBlockData::from_matrix(&[&[3.0, 3.0], &[3.0, 3.8]])
                    );

                    // Test falling rate
                    runtime.tick();
                    let inputs = Matrix {
                        data: [[2.0, 2.5], [1.5, 3.6]],
                    };
                    let output = block.process(&parameters, &runtime.context(), &inputs);
                    assert_eq!(
                        output,
                        &Matrix {
                            data: [[2.0, 2.5], [2.0, 3.6]],
                        }
                    );
                    assert_eq!(
                        block.data,
                        OldBlockData::from_matrix(&[&[2.0, 2.0], &[2.5, 3.6]])
                    );

                    // Test passing in no timestep does not change output
                    runtime.context.timestep = Some(Duration::from_secs(0));
                    let inputs = Matrix {
                        data: [[2.0, 2.5], [1.5, 3.6]],
                    };
                    let output = block.process(&parameters, &runtime.context(), &inputs);
                    assert_eq!(
                        output,
                        &Matrix {
                            data: [[2.0, 2.5], [2.0, 3.6]],
                        }
                    );
                    assert_eq!(
                        block.data,
                        OldBlockData::from_matrix(&[&[2.0, 2.0], &[2.5, 3.6]])
                    );
                }
            }
        };
    }

    impl_rate_limit_test!(f32);
    impl_rate_limit_test!(f64);
}
