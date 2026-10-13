use core::time::Duration;

use crate::{
    traits::{Float, MatrixOps},
    Scalar,
};
use pictorus_traits::{HasIc, Matrix, Pass, PassBy, ProcessBlock};

/// Performs a discrete integration of the input signal.
///
/// It can accept a scalar or a matrix input, for a matrix input it will do an
/// element wise integration.
pub struct IntegralBlock<T: Apply> {
    previous_sample: T::Output,
    output: T::Output,
}

impl<T: Apply> Default for IntegralBlock<T> {
    fn default() -> Self {
        const {
            panic!(
                "IntegralBlock has initial conditions and must be constructed with \
                 IntegralBlock::new(&parameters) (HasIc trait), not Default::default()."
            )
        }
    }
}

impl<F: Float, R: Scalar> ProcessBlock for IntegralBlock<(F, R)> {
    type Inputs = (F, R);
    type Output = F;
    type Parameters = Parameters<(F, R)>;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        model_clock: &dyn pictorus_traits::ModelClock,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        let (sample, reset) = inputs;
        if reset.is_truthy() {
            // Reset all state
            self.output = parameters.ic;
            self.previous_sample = parameters.ic;
        } else {
            let time_delta = F::from_duration(model_clock.timestep().unwrap_or(Duration::ZERO));
            let delta = match parameters.method {
                IntgeralMethod::Rectangle => integrate_rectangle_delta(sample, time_delta),
                IntgeralMethod::Trapezoidal => {
                    integrate_trapezoidal_delta(sample, self.previous_sample, time_delta)
                }
            };
            // Add delta to previous output
            let new_output = self.output + delta;

            // Check for clamping
            self.output = clamp(new_output, parameters.clamp_limit);
            //Store previous sample
            self.previous_sample = sample;
        }
        self.output.as_by()
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        self.output.as_by()
    }
}

impl<F: Float, R: Scalar> HasIc for IntegralBlock<(F, R)> {
    fn new(parameters: &Self::Parameters) -> Self {
        IntegralBlock::<(F, R)> {
            previous_sample: parameters.ic,
            output: parameters.ic,
        }
    }
}

impl<F: Float, const NROWS: usize, const NCOLS: usize, R: Scalar> ProcessBlock
    for IntegralBlock<(Matrix<NROWS, NCOLS, F>, R)>
{
    type Inputs = (Matrix<NROWS, NCOLS, F>, R);
    type Output = Matrix<NROWS, NCOLS, F>;
    type Parameters = Parameters<(Matrix<NROWS, NCOLS, F>, R)>;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        model_clock: &dyn pictorus_traits::ModelClock,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        let (sample, reset) = inputs;
        if reset.is_truthy() {
            // Reset all state
            self.output = parameters.ic;
            self.previous_sample = parameters.ic;
        } else {
            let time_delta = F::from_duration(model_clock.timestep().unwrap_or(Duration::ZERO));
            sample.for_each(|sample, c, r| {
                let delta = match parameters.method {
                    IntgeralMethod::Rectangle => integrate_rectangle_delta(sample, time_delta),
                    IntgeralMethod::Trapezoidal => integrate_trapezoidal_delta(
                        sample,
                        self.previous_sample.data[c][r],
                        time_delta,
                    ),
                };
                let new_output = self.output.data[c][r] + delta;
                self.output.data[c][r] = clamp(new_output, parameters.clamp_limit);
            });
            self.previous_sample = *sample;
        }
        self.output.as_by()
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        self.output.as_by()
    }
}

impl<F: Float, const NROWS: usize, const NCOLS: usize, R: Scalar> HasIc
    for IntegralBlock<(Matrix<NROWS, NCOLS, F>, R)>
{
    fn new(parameters: &Self::Parameters) -> Self {
        IntegralBlock::<(Matrix<NROWS, NCOLS, F>, R)> {
            previous_sample: parameters.ic,
            output: parameters.ic,
        }
    }
}

impl<F: Float, const NROWS: usize, const NCOLS: usize, R: Scalar> ProcessBlock
    for IntegralBlock<(Matrix<NROWS, NCOLS, F>, Matrix<NROWS, NCOLS, R>)>
{
    type Inputs = (Matrix<NROWS, NCOLS, F>, Matrix<NROWS, NCOLS, R>);
    type Output = Matrix<NROWS, NCOLS, F>;
    type Parameters = Parameters<(Matrix<NROWS, NCOLS, F>, Matrix<NROWS, NCOLS, R>)>;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        model_clock: &dyn pictorus_traits::ModelClock,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        let (sample, reset) = inputs;
        let time_delta = F::from_duration(model_clock.timestep().unwrap_or(Duration::ZERO));
        sample.for_each(|sample, c, r| {
            if reset.data[c][r].is_truthy() {
                // Reset this elements state
                self.output.data[c][r] = parameters.ic.data[c][r];
                self.previous_sample.data[c][r] = parameters.ic.data[c][r];
            } else {
                let delta = match parameters.method {
                    IntgeralMethod::Rectangle => integrate_rectangle_delta(sample, time_delta),
                    IntgeralMethod::Trapezoidal => integrate_trapezoidal_delta(
                        sample,
                        self.previous_sample.data[c][r],
                        time_delta,
                    ),
                };
                let new_output = self.output.data[c][r] + delta;
                self.output.data[c][r] = clamp(new_output, parameters.clamp_limit);
                self.previous_sample.data[c][r] = sample;
            }
        });
        self.output.as_by()
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        self.output.as_by()
    }
}

impl<F: Float, const NROWS: usize, const NCOLS: usize, R: Scalar> HasIc
    for IntegralBlock<(Matrix<NROWS, NCOLS, F>, Matrix<NROWS, NCOLS, R>)>
{
    fn new(parameters: &Self::Parameters) -> Self {
        IntegralBlock::<(Matrix<NROWS, NCOLS, F>, Matrix<NROWS, NCOLS, R>)> {
            previous_sample: parameters.ic,
            output: parameters.ic,
        }
    }
}

fn integrate_rectangle_delta<F: Float>(sample: F, timestep: F) -> F {
    timestep * sample
}

fn integrate_trapezoidal_delta<F: Float>(sample: F, previous_sample: F, timestep: F) -> F {
    timestep * (sample + previous_sample) / (F::one() + F::one())
}

fn clamp<F: Float>(value: F, limit: F) -> F {
    if value > limit {
        limit
    } else if value < -limit {
        -limit
    } else {
        value
    }
}

pub trait Apply: Pass + Default {
    type Output: Pass + Default;
    type Float;
}

impl<F: Float, R: Scalar> Apply for (F, R) {
    type Output = F;
    type Float = F;
}

impl<F: Float, const NROWS: usize, const NCOLS: usize, R: Scalar> Apply
    for (Matrix<NROWS, NCOLS, F>, R)
{
    type Output = Matrix<NROWS, NCOLS, F>;
    type Float = F;
}

impl<F: Float, const NROWS: usize, const NCOLS: usize, R: Scalar> Apply
    for (Matrix<NROWS, NCOLS, F>, Matrix<NROWS, NCOLS, R>)
{
    type Output = Matrix<NROWS, NCOLS, F>;
    type Float = F;
}

/// Controls the method of integration
/// Rectangle: Uses the value of the sample at the current time step and the time step duration to calculate the integral.
/// Trapezoidal: Uses the average of the sample at the current time step and the previous time step and the time step duration to calculate the integral.
#[derive(strum::EnumString, Debug, Clone, Copy)]
pub enum IntgeralMethod {
    Rectangle,
    Trapezoidal,
}

pub struct Parameters<T: Apply> {
    pub clamp_limit: T::Float,
    pub ic: T::Output,
    pub method: IntgeralMethod,
}

/// Parameters for Integral Block
/// ic: Initial condition, the value of the integral at the start of the simulation
/// clamp_limit: Maximum absolute value of the integral (or each element of the integral in case of matrix input)
/// method: Method of integration (See [`IntgeralMethod`])
impl<T: Apply> Parameters<T> {
    pub fn new(ic: T::Output, clamp_limit: T::Float, method: &str) -> Self {
        Parameters {
            clamp_limit,
            ic,
            method: method.parse().unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{StubModelClock, StubRuntime};
    use crate::SinewaveBlock;
    use approx::assert_relative_eq;
    use pictorus_traits::GeneratorBlock;

    #[test]
    fn test_integral_scalar() {
        let mut runtime = StubRuntime::default();

        let parameters = Parameters::new(0.0, 20.0, "Trapezoidal");
        let mut block = IntegralBlock::<(f64, bool)>::new(&parameters);

        let mut sine_wave_block = SinewaveBlock::<f64>::default();
        let sine_wave_parameters =
            <SinewaveBlock<f64> as GeneratorBlock>::Parameters::new(1.0, 1.0, 0.0, 0.0);

        let mut cosine_wave_block = SinewaveBlock::<f64>::default();
        // Cosine is sinewave offset with pi/2 phase shift
        let cosine_wave_parameters =
            <SinewaveBlock<f64> as GeneratorBlock>::Parameters::new(1.0, 1.0, f64::PI / 2., 0.0);

        for _ in 0..100 {
            let sine_output =
                sine_wave_block.generate(&sine_wave_parameters, &runtime.model_clock());
            let cosine_output =
                cosine_wave_block.generate(&cosine_wave_parameters, &runtime.model_clock());
            runtime.tick();

            let integral_output = block.process(
                &parameters,
                &runtime.model_clock(),
                (cosine_output, false).as_by(),
            );
            // Integral of cosine is sine, with a small offset and allowing discrete tolerance.
            assert_relative_eq!(integral_output, sine_output + 0.05, epsilon = 0.01);
        }

        // Reset with any input value
        let reset_output = block.process(
            &parameters,
            &runtime.model_clock(),
            (1000000.0, true).as_by(),
        );
        assert_relative_eq!(reset_output, 0.0, epsilon = 0.01);
        assert_relative_eq!(block.buffer(), reset_output);
    }

    #[test]
    fn test_integral_matrix_scalar() {
        let mut runtime = StubRuntime::new(StubModelClock::new(
            Duration::ZERO,
            None,
            Duration::from_secs(1),
        ));

        let parameters: Parameters<(Matrix<1, 3, f64>, bool)> = Parameters::new(
            Matrix {
                data: [[0.0], [0.0], [0.0]],
            },
            20.0,
            "Rectangle",
        );
        let mut block: IntegralBlock<(Matrix<1, 3, f64>, bool)> =
            IntegralBlock::<(Matrix<1, 3, f64>, bool)>::new(&parameters);

        let input = Matrix {
            data: [[1.0], [1.0], [15.0]],
        };

        let output = block.process(&parameters, &runtime.model_clock(), (input, false).as_by());
        assert_eq!(output.data, [[0.0], [0.0], [0.0]]);
        runtime.tick();
        let output = block.process(&parameters, &runtime.model_clock(), (input, false).as_by());
        assert_eq!(output.data, [[1.0], [1.0], [15.0]]);

        runtime.tick();
        let output = block.process(&parameters, &runtime.model_clock(), (input, false).as_by());
        // Hits clamp limit
        assert_eq!(output.data, [[2.0], [2.0], [20.0]]);
    }

    #[test]
    fn test_integral_matrix_matrix() {
        let mut runtime = StubRuntime::new(StubModelClock::new(
            Duration::ZERO,
            None,
            Duration::from_secs(1),
        ));

        let parameters: Parameters<(Matrix<1, 3, f64>, Matrix<1, 3, bool>)> = Parameters::new(
            Matrix {
                data: [[0.0], [0.0], [0.0]],
            },
            20.0,
            "Rectangle",
        );
        let mut block: IntegralBlock<(Matrix<1, 3, f64>, Matrix<1, 3, bool>)> =
            IntegralBlock::<(Matrix<1, 3, f64>, Matrix<1, 3, bool>)>::new(&parameters);

        let input = (
            Matrix {
                data: [[1.0], [1.0], [15.0]],
            },
            Matrix {
                data: [[false], [false], [false]],
            },
        );
        let output = block.process(&parameters, &runtime.model_clock(), input.as_by());
        assert_eq!(output.data, [[0.0], [0.0], [0.0]]);
        runtime.tick();
        let output = block.process(&parameters, &runtime.model_clock(), input.as_by());
        assert_eq!(output.data, [[1.0], [1.0], [15.0]]);

        runtime.tick();
        let output = block.process(&parameters, &runtime.model_clock(), input.as_by());
        // Hits clamp limit
        assert_eq!(output.data, [[2.0], [2.0], [20.0]]);
    }

    #[test]
    fn test_integral_ic_scalar() {
        let mut runtime = StubRuntime::new(StubModelClock::new(
            Duration::ZERO,
            None,
            Duration::from_secs(1),
        ));

        let parameters = Parameters::new(10.0, 50.0, "Rectangle");
        let mut block = IntegralBlock::<(f64, bool)>::new(&parameters);
        // Check the initial value is set
        assert_eq!(block.buffer(), 10.0);

        let input = 25.0;
        let output = block.process(&parameters, &runtime.model_clock(), (input, false).as_by());
        // This is pretty confusing, but the dt of the first tick is 0, so the output is the same as the IC
        assert_eq!(output, 10.0);
        runtime.tick();
        let output = block.process(&parameters, &runtime.model_clock(), (input, false).as_by());
        assert_eq!(output, 35.0);

        runtime.tick();
        let output = block.process(&parameters, &runtime.model_clock(), (input, false).as_by());
        assert_eq!(output, 50.0);
    }

    #[test]
    fn test_integral_ic_matrix_scalar_reset() {
        let mut runtime = StubRuntime::new(StubModelClock::new(
            Duration::ZERO,
            None,
            Duration::from_secs(1),
        ));

        let parameters = Parameters::new(
            Matrix {
                data: [[10.0], [10.0], [10.0]],
            },
            50.0,
            "Rectangle",
        );
        let mut block = IntegralBlock::<(Matrix<1, 3, f64>, bool)>::new(&parameters);
        // Check the initial value is set
        assert_eq!(block.buffer().data, [[10.0], [10.0], [10.0]]);

        let input = Matrix {
            data: [[1.0], [1.0], [25.0]],
        };
        let output = block.process(&parameters, &runtime.model_clock(), (input, false).as_by());
        assert_eq!(output.data, [[10.0], [10.0], [10.0]]);
        runtime.tick();
        let output = block.process(&parameters, &runtime.model_clock(), (input, false).as_by());
        assert_eq!(output.data, [[11.0], [11.0], [35.0]]);

        runtime.tick();
        let output = block.process(&parameters, &runtime.model_clock(), (input, false).as_by());
        // Hits clamp limit
        assert_eq!(output.data, [[12.0], [12.0], [50.0]]);

        runtime.tick();
        let output = block.process(&parameters, &runtime.model_clock(), (input, true).as_by());
        // After a reset, the output should return to the initial condition
        assert_eq!(output.data, [[10.0], [10.0], [10.0]]);
    }

    #[test]
    fn test_integral_ic_matrix_matrix_reset() {
        let mut runtime = StubRuntime::new(StubModelClock::new(
            Duration::ZERO,
            None,
            Duration::from_secs(1),
        ));

        let parameters = Parameters::new(
            Matrix {
                data: [[10.0], [10.0], [10.0]],
            },
            50.0,
            "Rectangle",
        );
        let mut block = IntegralBlock::<(Matrix<1, 3, f64>, Matrix<1, 3, bool>)>::new(&parameters);
        // Check the initial value is set
        assert_eq!(block.buffer().data, [[10.0], [10.0], [10.0]]);

        let input = (
            Matrix {
                data: [[1.0], [1.0], [25.0]],
            },
            Matrix {
                data: [[false], [false], [false]],
            },
        );
        let output = block.process(&parameters, &runtime.model_clock(), input.as_by());
        assert_eq!(output.data, [[10.0], [10.0], [10.0]]);
        runtime.tick();
        let output = block.process(&parameters, &runtime.model_clock(), input.as_by());
        assert_eq!(output.data, [[11.0], [11.0], [35.0]]);

        runtime.tick();
        let output = block.process(&parameters, &runtime.model_clock(), input.as_by());
        // Hits clamp limit
        assert_eq!(output.data, [[12.0], [12.0], [50.0]]);

        runtime.tick();
        let output = block.process(&parameters, &runtime.model_clock(), (
            Matrix {
                data: [[1.0], [1.0], [25.0]],
            },
            Matrix {
                data: [[false], [true], [false]],
            },
        ).as_by());
        // Partial reset, the reset outputs should return to the initial condition
        assert_eq!(output.data, [[13.0], [10.0], [50.0]]);
    }
}
