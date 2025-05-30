use crate::traits::Float;
use crate::traits::{MatrixOps, Scalar};
use core::time::Duration;
use pictorus_block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{HasIc, Matrix, Pass, PassBy, ProcessBlock};

/// Perform a frequency filter operation on a signal, either as a low pass or high pass filter.
///
/// The filter is implemented as a first order RC filter.
/// The cutoff frequency and filter type are configurable via paramaters.
///
/// This block can accept a scalar or a matrix input. For a matrix input, the filter is applied
/// independently to each element of the matrix.
#[derive(Debug)]
pub struct FrequencyFilterBlock<T: Pass> {
    pub data: OldBlockData,
    prev_data: Option<PreviousData<T>>,
    output: T,
}

#[derive(Debug, Clone)]
struct PreviousData<T: Pass> {
    prev_input: T,
    prev_output: T,
    prev_time: Duration,
}

impl<T: Pass + Default> Default for FrequencyFilterBlock<T>
where
    OldBlockData: FromPass<T>,
{
    fn default() -> Self {
        Self {
            data: <OldBlockData as FromPass<T>>::from_pass(T::default().as_by()),
            prev_data: None,
            output: T::default(),
        }
    }
}

impl<T> HasIc for FrequencyFilterBlock<T>
where
    T: Pass + Default + Float,
    OldBlockData: FromPass<T>,
{
    fn new(parameters: &Self::Parameters) -> Self {
        FrequencyFilterBlock::<T> {
            prev_data: Some(PreviousData {
                prev_input: parameters.ic,
                prev_output: parameters.ic,
                prev_time: Duration::ZERO,
            }),
            data: OldBlockData::from_pass(parameters.ic.as_by()),
            output: parameters.ic,
        }
    }
}

impl<T, const NROWS: usize, const NCOLS: usize> HasIc
    for FrequencyFilterBlock<Matrix<NROWS, NCOLS, T>>
where
    T: Pass + Default + Float + Scalar,
    OldBlockData: FromPass<Matrix<NROWS, NCOLS, T>>,
{
    fn new(parameters: &Self::Parameters) -> Self {
        FrequencyFilterBlock::<pictorus_traits::Matrix<NROWS, NCOLS, T>> {
            prev_data: Some(PreviousData {
                prev_input: parameters.ic,
                prev_output: parameters.ic,
                prev_time: Duration::ZERO,
            }),
            data: OldBlockData::from_pass(parameters.ic.as_by()),
            output: parameters.ic,
        }
    }
}

impl<T> ProcessBlock for FrequencyFilterBlock<T>
where
    T: Pass + Default + Float,
    OldBlockData: FromPass<T>,
{
    type Inputs = T;
    type Output = T;
    type Parameters = Parameters<T, T>;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        if let Some(previous_data) = &self.prev_data {
            let timestep = context.time() - previous_data.prev_time;
            let alpha = compute_alpha(parameters.method, parameters.cutoff_frequency, timestep);
            self.output = match parameters.method {
                FrequencyFilterEnum::HighPass => {
                    alpha * (previous_data.prev_output + inputs - previous_data.prev_input)
                }
                FrequencyFilterEnum::LowPass => {
                    previous_data.prev_output + alpha * (inputs - previous_data.prev_output)
                }
            };
        } else {
            self.output = inputs;
        }
        let _ = self.prev_data.insert(PreviousData {
            prev_input: inputs,
            prev_output: self.output,
            prev_time: context.time(),
        });
        self.data = OldBlockData::from_pass(self.output.as_by());
        self.output.as_by()
    }
}

impl<T, const NROWS: usize, const NCOLS: usize> ProcessBlock
    for FrequencyFilterBlock<Matrix<NROWS, NCOLS, T>>
where
    T: Pass + Default + Float + Scalar,
    OldBlockData: FromPass<Matrix<NROWS, NCOLS, T>>,
{
    type Inputs = Matrix<NROWS, NCOLS, T>;
    type Output = Matrix<NROWS, NCOLS, T>;
    type Parameters = Parameters<Matrix<NROWS, NCOLS, T>, T>;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        if let Some(previous_data) = &self.prev_data {
            let timestep = context.time() - previous_data.prev_time;
            let alpha = compute_alpha(parameters.method, parameters.cutoff_frequency, timestep);
            inputs.for_each(|input, col, row| {
                let prev_output = previous_data.prev_output.data[col][row];
                let prev_input = previous_data.prev_input.data[col][row];
                let output = match parameters.method {
                    FrequencyFilterEnum::HighPass => alpha * (prev_output + input - prev_input),
                    FrequencyFilterEnum::LowPass => prev_output + alpha * (input - prev_output),
                };
                self.output.data[col][row] = output;
            });
        } else {
            self.output.data = inputs.data;
        }

        let _ = self.prev_data.insert(PreviousData {
            prev_input: Matrix { data: inputs.data },
            prev_output: self.output,
            prev_time: context.time(),
        });
        self.data = OldBlockData::from_pass(self.output.as_by());
        self.output.as_by()
    }
}

fn compute_alpha<T: Scalar + Float>(
    method: FrequencyFilterEnum,
    cutoff_frequency: T,
    timestep: Duration,
) -> T {
    let timestep_s: T = T::from_duration(timestep);
    match method {
        FrequencyFilterEnum::HighPass => {
            T::one() / (T::one() + (T::TAU * cutoff_frequency * timestep_s))
        }
        FrequencyFilterEnum::LowPass => {
            (T::TAU * cutoff_frequency * timestep_s)
                / (T::one() + (T::TAU * cutoff_frequency * timestep_s))
        }
    }
}

/// Parameters for the FrequencyFilterBlock
#[derive(Debug, Clone, Copy)]
pub struct Parameters<T, C: Float> {
    /// Frequency in Hz of the filter cutoff
    pub cutoff_frequency: C,
    /// Filter Type
    pub method: FrequencyFilterEnum,
    ic: T,
}

impl<T: Pass, C: Float> Parameters<T, C> {
    pub fn new(ic: T, cutoff_frequency: C, method: &str) -> Self {
        Self {
            ic,
            cutoff_frequency,
            method: method.parse().unwrap(),
        }
    }
}

/// Enum for the type of filter
#[derive(strum::EnumString, Clone, Copy, Debug)]
pub enum FrequencyFilterEnum {
    /// High Pass Filter
    HighPass,
    /// Low Pass Filter
    LowPass,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_blocks::{sinewave_block::Parameters as SinewaveParameters, SinewaveBlock};
    use crate::testing::StubRuntime;
    use pictorus_traits::GeneratorBlock;
    use std::vec::Vec;

    fn rms<T: Float + core::iter::Sum>(data: &[T]) -> T {
        let sum: T = data.iter().map(|x| *x * *x).sum();
        num_traits::Float::sqrt(sum / T::from(data.len()).unwrap())
    }

    #[test]
    fn test_freq_filter_high_pass_scalar() {
        let mut runtime = StubRuntime::default();
        runtime.context.fundamental_timestep = Duration::from_secs_f64(0.001);
        let mut block_1_hz = FrequencyFilterBlock::<f64>::default();
        let parameters_1_hz = Parameters::new(0.0, 1.0, "HighPass");
        let mut block_50_hz = FrequencyFilterBlock::<f64>::default();
        let parameters_50_hz = Parameters::new(0.0, 50.0, "HighPass");

        let mut sinewave_25_hz = SinewaveBlock::default();
        let sinewave_25_hz_parameters = SinewaveParameters::new(1.0, 25.0 * f64::TAU, 0.0, 0.0);

        let mut sine_data = [0.0; 1000];
        let mut high_pass_1_hz_data = [0.0; 1000];
        let mut high_pass_50_hz_data = [0.0; 1000];

        for i in 0..1000 {
            sine_data[i] = sinewave_25_hz.generate(&sinewave_25_hz_parameters, &runtime.context());
            high_pass_1_hz_data[i] =
                block_1_hz.process(&parameters_1_hz, &runtime.context(), sine_data[i].as_by());
            high_pass_50_hz_data[i] =
                block_50_hz.process(&parameters_50_hz, &runtime.context(), sine_data[i].as_by());
            runtime.tick();
        }

        let rms_sine = rms(&sine_data);
        let rms_high_pass_1_hz = rms(&high_pass_1_hz_data);
        let rms_high_pass_50_hz = rms(&high_pass_50_hz_data);
        // Assert the the 1 HZ highpassed signal is within 5% of the original signal since the 25 Hz is well above the cutoff fequency
        assert!((rms_sine - rms_high_pass_1_hz).abs() / rms_sine < 0.05);
        //Assert that the 50Hz highpassed signal is within 50% of the original signal since the 25 Hz is below the cutoff frequency and it has started to roll off
        assert!(rms_high_pass_50_hz < 0.5 * rms_sine);
    }

    #[test]
    fn test_freq_filter_low_pass_scalar() {
        let mut runtime = StubRuntime::default();
        runtime.context.fundamental_timestep = Duration::from_secs_f64(0.001);
        let mut block_1_hz = FrequencyFilterBlock::<f32>::default();
        let parameters_1_hz = Parameters::new(0.0, 1.0, "LowPass");
        let mut block_50_hz = FrequencyFilterBlock::<f32>::default();
        let parameters_50_hz = Parameters::new(0.0, 50.0, "LowPass");

        let mut sinewave_25_hz = SinewaveBlock::default();
        let sinewave_25_hz_parameters = SinewaveParameters::new(1.0, 25.0 * f32::TAU, 0.0, 0.0);

        let mut sine_data = [0.0; 1000];
        let mut low_pass_1_hz_data = [0.0; 1000];
        let mut low_pass_50_hz_data = [0.0; 1000];

        for i in 0..1000 {
            sine_data[i] = sinewave_25_hz.generate(&sinewave_25_hz_parameters, &runtime.context());
            low_pass_1_hz_data[i] =
                block_1_hz.process(&parameters_1_hz, &runtime.context(), sine_data[i].as_by());
            low_pass_50_hz_data[i] =
                block_50_hz.process(&parameters_50_hz, &runtime.context(), sine_data[i].as_by());
            runtime.tick();
        }

        let rms_sine = rms(&sine_data);
        let rms_low_pass_1_hz = rms(&low_pass_1_hz_data);
        let rms_low_pass_50_hz = rms(&low_pass_50_hz_data);

        // Assert the the 1 HZ lowpassed signal is less than 5% of the original signal since the 25 Hz is well above the cutoff fequency
        assert!(rms_low_pass_1_hz < 0.05 * rms_sine);
        //Assert that the 50Hz lowpassed signal is within 85% of the original signal since the 25 Hz is below the cutoff frequency
        assert!((rms_sine - rms_low_pass_50_hz).abs() / rms_sine < 0.15);
    }

    #[test]
    fn test_freq_filter_high_pass_matrix() {
        let mut runtime = StubRuntime::default();
        runtime.context.fundamental_timestep = Duration::from_secs_f64(0.001);

        let params_50_hz_high_pass_matrix = Parameters::new(Matrix::zeroed(), 50.0, "HighPass");
        let params_50_hz_high_pass_scalar = Parameters::new(0.0, 50.0, "HighPass");

        // 4 sine generators
        let mut sinewave_generators: Vec<_> = (0..4).map(|_| SinewaveBlock::default()).collect();
        let params_sinewave_generators: Vec<_> = (1..5)
            .map(f64::from)
            .map(|x| f64::TAU * x * 10.0)
            .map(|freq| SinewaveParameters::new(1.0, freq, 0.0, 0.0))
            .collect();

        // 4 scalar filters
        let mut high_pass_50_hz_scalars: Vec<FrequencyFilterBlock<f64>> =
            (0..4).map(|_| FrequencyFilterBlock::default()).collect();

        //Matrix filters
        let mut high_pass_50hz_mat: FrequencyFilterBlock<Matrix<2, 2, f64>> =
            FrequencyFilterBlock::default();

        for _ in 0..1000 {
            let sine_data = [
                [
                    sinewave_generators[0]
                        .generate(&params_sinewave_generators[0], &runtime.context()),
                    sinewave_generators[1]
                        .generate(&params_sinewave_generators[1], &runtime.context()),
                ],
                [
                    sinewave_generators[2]
                        .generate(&params_sinewave_generators[2], &runtime.context()),
                    sinewave_generators[3]
                        .generate(&params_sinewave_generators[3], &runtime.context()),
                ],
            ];

            let hp_scalar_data = [
                [
                    high_pass_50_hz_scalars[0].process(
                        &params_50_hz_high_pass_scalar,
                        &runtime.context(),
                        sine_data[0][0].as_by(),
                    ),
                    high_pass_50_hz_scalars[1].process(
                        &params_50_hz_high_pass_scalar,
                        &runtime.context(),
                        sine_data[0][1].as_by(),
                    ),
                ],
                [
                    high_pass_50_hz_scalars[2].process(
                        &params_50_hz_high_pass_scalar,
                        &runtime.context(),
                        sine_data[1][0].as_by(),
                    ),
                    high_pass_50_hz_scalars[3].process(
                        &params_50_hz_high_pass_scalar,
                        &runtime.context(),
                        sine_data[1][1].as_by(),
                    ),
                ],
            ];

            let hp_mat_data: &Matrix<2, 2, f64> = high_pass_50hz_mat.process(
                &params_50_hz_high_pass_matrix,
                &runtime.context,
                &Matrix { data: sine_data },
            );
            assert_eq!(hp_mat_data.data, hp_scalar_data);
            runtime.tick();
        }
    }

    #[test]
    fn test_freq_filter_low_pass_matrix() {
        let mut runtime = StubRuntime::default();
        runtime.context.fundamental_timestep = Duration::from_secs_f64(0.001);

        let params_50_hz_low_pass_matrix = Parameters::new(Matrix::zeroed(), 50.0, "LowPass");
        let params_50_hz_low_pass_scalar = Parameters::new(0.0, 50.0, "LowPass");

        // 4 sine generators
        let mut sinewave_generators: Vec<_> = (0..4).map(|_| SinewaveBlock::default()).collect();
        let params_sinewave_generators: Vec<_> = (1..5)
            .map(f64::from)
            .map(|x| f64::TAU * x * 10.0)
            .map(|freq| SinewaveParameters::new(1.0, freq, 0.0, 0.0))
            .collect();

        // 4 scalar filters
        let mut low_pass_50_hz_scalars: Vec<FrequencyFilterBlock<f64>> =
            (0..4).map(|_| FrequencyFilterBlock::default()).collect();

        //Matrix filters
        let mut low_pass_50hz_mat: FrequencyFilterBlock<Matrix<2, 2, f64>> =
            FrequencyFilterBlock::default();

        for _ in 0..1000 {
            let sine_data = [
                [
                    sinewave_generators[0]
                        .generate(&params_sinewave_generators[0], &runtime.context()),
                    sinewave_generators[1]
                        .generate(&params_sinewave_generators[1], &runtime.context()),
                ],
                [
                    sinewave_generators[2]
                        .generate(&params_sinewave_generators[2], &runtime.context()),
                    sinewave_generators[3]
                        .generate(&params_sinewave_generators[3], &runtime.context()),
                ],
            ];

            let hp_scalar_data = [
                [
                    low_pass_50_hz_scalars[0].process(
                        &params_50_hz_low_pass_scalar,
                        &runtime.context(),
                        sine_data[0][0].as_by(),
                    ),
                    low_pass_50_hz_scalars[1].process(
                        &params_50_hz_low_pass_scalar,
                        &runtime.context(),
                        sine_data[0][1].as_by(),
                    ),
                ],
                [
                    low_pass_50_hz_scalars[2].process(
                        &params_50_hz_low_pass_scalar,
                        &runtime.context(),
                        sine_data[1][0].as_by(),
                    ),
                    low_pass_50_hz_scalars[3].process(
                        &params_50_hz_low_pass_scalar,
                        &runtime.context(),
                        sine_data[1][1].as_by(),
                    ),
                ],
            ];

            let hp_mat_data: &Matrix<2, 2, f64> = low_pass_50hz_mat.process(
                &params_50_hz_low_pass_matrix,
                &runtime.context,
                &Matrix { data: sine_data },
            );
            assert_eq!(hp_mat_data.data, hp_scalar_data);
            runtime.tick();
        }
    }
}
