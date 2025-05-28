use core::time::Duration;

use crate::traits::{Float, MatrixOps};
use block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{HasIc, Matrix, Pass, PassBy, ProcessBlock};

/// Integral Block performs integration of input signal.
/// It can accept a scalar or a matrix input, for a matrix input it will do an
/// element wise integration.
pub struct IntegralBlock<T: Apply>
where
    OldBlockData: FromPass<T>,
{
    previous_sample: Option<T>,
    output: Option<T>,
    pub data: OldBlockData,
}

impl<T: Apply> Default for IntegralBlock<T>
where
    OldBlockData: FromPass<T>,
{
    fn default() -> Self {
        Self {
            previous_sample: None,
            output: None,
            data: <OldBlockData as FromPass<T>>::from_pass(T::default().as_by()),
        }
    }
}

impl<F: Float> ProcessBlock for IntegralBlock<F>
where
    OldBlockData: FromPass<F>,
{
    type Inputs = (F, F);
    type Output = F;
    type Parameters = Parameters<F>;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        let (sample, reset) = inputs;
        if reset.is_truthy() {
            // Reset all state
            self.output = None;
            self.previous_sample = None;
        } else {
            let delta = match parameters.method {
                IntgeralMethod::Rectangle => {
                    F::from_duration(context.timestep().unwrap_or(Duration::ZERO)) * sample
                }
                IntgeralMethod::Trapezoidal => {
                    F::from_duration(context.timestep().unwrap_or(Duration::ZERO))
                        * (sample + self.previous_sample.unwrap_or(parameters.ic))
                        / (F::one() + F::one())
                }
            };
            // Add delta to previous output, If output was None (i.e. the very first run, resets don't count) default to ic
            let output = self
                .output
                .insert(self.output.unwrap_or(parameters.ic) + delta);

            // Check for clamping
            if *output > parameters.clamp_limit {
                *output = parameters.clamp_limit;
            } else if *output < -parameters.clamp_limit {
                *output = -parameters.clamp_limit;
            }
            //Store previous sample
            self.previous_sample = Some(sample);
        }
        // This could be none if we were reset above
        let output = self.output.get_or_insert(parameters.ic);
        self.data = OldBlockData::from_pass(output.as_by());
        output.as_by()
    }
}

impl<F: Float> HasIc for IntegralBlock<F>
where
    OldBlockData: FromPass<F>,
{
    fn new(parameters: &Self::Parameters) -> Self {
        IntegralBlock::<F> {
            previous_sample: None,
            output: Some(parameters.ic),
            data: <OldBlockData as FromPass<F>>::from_pass(parameters.ic.as_by()),
        }
    }
}

impl<F: Float, const NROWS: usize, const NCOLS: usize> ProcessBlock
    for IntegralBlock<Matrix<NROWS, NCOLS, F>>
where
    OldBlockData: FromPass<Matrix<NROWS, NCOLS, F>>,
{
    type Inputs = (Matrix<NROWS, NCOLS, F>, F);
    type Output = Matrix<NROWS, NCOLS, F>;
    type Parameters = Parameters<Matrix<NROWS, NCOLS, F>>;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        let (sample, reset) = inputs;
        if reset.is_truthy() {
            self.output = None;
            self.previous_sample = None;
        } else {
            let output = self.output.get_or_insert(parameters.ic);
            sample.for_each(|sample, c, r| {
                let delta = match parameters.method {
                    IntgeralMethod::Rectangle => {
                        F::from_duration(context.timestep().unwrap_or(Duration::ZERO)) * sample
                    }
                    IntgeralMethod::Trapezoidal => {
                        F::from_duration(context.timestep().unwrap_or(Duration::ZERO))
                            * (sample + self.previous_sample.unwrap_or(parameters.ic).data[c][r])
                            / (F::one() + F::one())
                    }
                };
                output.data[c][r] += delta;
                if output.data[c][r] > parameters.clamp_limit {
                    output.data[c][r] = parameters.clamp_limit;
                } else if output.data[c][r] < -parameters.clamp_limit {
                    output.data[c][r] = -parameters.clamp_limit;
                }
            });
            self.previous_sample = Some(*sample);
        }
        let output = self.output.get_or_insert(parameters.ic);
        self.data = OldBlockData::from_pass(output);
        output
    }
}

impl<F: Float, const NROWS: usize, const NCOLS: usize> HasIc
    for IntegralBlock<Matrix<NROWS, NCOLS, F>>
where
    OldBlockData: FromPass<Matrix<NROWS, NCOLS, F>>,
{
    fn new(parameters: &Self::Parameters) -> Self {
        IntegralBlock::<Matrix<NROWS, NCOLS, F>> {
            previous_sample: None,
            output: Some(parameters.ic),
            data: <OldBlockData as FromPass<Matrix<NROWS, NCOLS, F>>>::from_pass(&parameters.ic),
        }
    }
}

pub trait Apply: Pass + Default {
    type Float: Float;
}

impl<F: Float> Apply for F {
    type Float = F;
}

impl<F: Float, const NROWS: usize, const NCOLS: usize> Apply for Matrix<NROWS, NCOLS, F> {
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
    pub ic: T,
    pub method: IntgeralMethod,
}

/// Parameters for Integral Block
/// ic: Initial condition, the value of the integral at the start of the simulation
/// clamp_limit: Maximum absolute value of the integral (or each element of the integral in case of matrix input)
/// method: Method of integration (See [`IntgeralMethod`])
impl<T: Apply> Parameters<T> {
    pub fn new(ic: T, clamp_limit: T::Float, method: &str) -> Self {
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
    use crate::testing::{StubContext, StubRuntime};
    use crate::SinewaveBlock;
    use approx::assert_relative_eq;
    use pictorus_traits::GeneratorBlock;

    #[test]
    fn test_integral_scalar() {
        let mut runtime = StubRuntime::default();

        let mut block = IntegralBlock::<f64>::default();
        let parameters = Parameters::new(0.0, 20.0, "Trapezoidal");

        let mut sine_wave_block = SinewaveBlock::<f64>::default();
        let sine_wave_parameters =
            <SinewaveBlock<f64> as GeneratorBlock>::Parameters::new(1.0, 1.0, 0.0, 0.0);

        let mut cosine_wave_block = SinewaveBlock::<f64>::default();
        // Cosine is sinewave offset with pi/2 phase shift
        let cosine_wave_parameters =
            <SinewaveBlock<f64> as GeneratorBlock>::Parameters::new(1.0, 1.0, f64::PI / 2., 0.0);

        for _ in 0..100 {
            let sine_output = sine_wave_block.generate(&sine_wave_parameters, &runtime.context());
            let cosine_output =
                cosine_wave_block.generate(&cosine_wave_parameters, &runtime.context());
            runtime.tick();

            let integral_output = block.process(
                &parameters,
                &runtime.context(),
                (cosine_output, false).as_by(),
            );
            // Integral of cosine is sine, with a small offset and allowing discrete tolerance.
            assert_relative_eq!(integral_output, sine_output + 0.05, epsilon = 0.01);
        }

        // Reset with any input value
        let reset_output =
            block.process(&parameters, &runtime.context(), (1000000.0, true).as_by());
        assert_relative_eq!(reset_output, 0.0, epsilon = 0.01);
    }

    #[test]
    fn test_integral_matrix() {
        let mut runtime = StubRuntime::new(StubContext::new(
            Duration::ZERO,
            None,
            Duration::from_secs(1),
        ));

        let mut block = IntegralBlock::<Matrix<1, 3, f64>>::default();
        let parameters = Parameters::new(
            Matrix {
                data: [[0.0], [0.0], [0.0]],
            },
            20.0,
            "Rectangle",
        );

        let input = Matrix {
            data: [[1.0], [1.0], [15.0]],
        };

        let output = block.process(&parameters, &runtime.context(), (input, false).as_by());
        assert_eq!(output.data, [[0.0], [0.0], [0.0]]);
        runtime.tick();
        let output = block.process(&parameters, &runtime.context(), (input, false).as_by());
        assert_eq!(output.data, [[1.0], [1.0], [15.0]]);

        runtime.tick();
        let output = block.process(&parameters, &runtime.context(), (input, false).as_by());
        // Hits clamp limit
        assert_eq!(output.data, [[2.0], [2.0], [20.0]]);
    }

    #[test]
    fn test_integral_ic_scalar() {
        let mut runtime = StubRuntime::new(StubContext::new(
            Duration::ZERO,
            None,
            Duration::from_secs(1),
        ));

        let parameters = Parameters::new(10.0, 50.0, "Rectangle");
        let mut block = IntegralBlock::<f64>::new(&parameters);
        // Check the initial value is set
        assert_eq!(block.data.scalar(), 10.0);

        let input = 25.0;
        let output = block.process(&parameters, &runtime.context(), (input, false).as_by());
        // This is pretty confusing, but the dt of the first tick is 0, so the output is the same as the IC
        assert_eq!(output, 10.0);
        runtime.tick();
        let output = block.process(&parameters, &runtime.context(), (input, false).as_by());
        assert_eq!(output, 35.0);

        runtime.tick();
        let output = block.process(&parameters, &runtime.context(), (input, false).as_by());
        assert_eq!(output, 50.0);
    }

    #[test]
    fn test_integral_ic_matrix() {
        let mut runtime = StubRuntime::new(StubContext::new(
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
        let mut block = IntegralBlock::<Matrix<1, 3, f64>>::new(&parameters);
        // Check the initial value is set
        assert_eq!(
            block.data.get_data().as_slice(),
            [[10.0], [10.0], [10.0]].as_flattened()
        );

        let input = Matrix {
            data: [[1.0], [1.0], [25.0]],
        };
        let output = block.process(&parameters, &runtime.context(), (input, false).as_by());
        assert_eq!(output.data, [[10.0], [10.0], [10.0]]);
        runtime.tick();
        let output = block.process(&parameters, &runtime.context(), (input, false).as_by());
        assert_eq!(output.data, [[11.0], [11.0], [35.0]]);

        runtime.tick();
        let output = block.process(&parameters, &runtime.context(), (input, false).as_by());
        // Hits clamp limit
        assert_eq!(output.data, [[12.0], [12.0], [50.0]]);
    }
}
