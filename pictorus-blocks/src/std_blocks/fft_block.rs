use crate::traits::Float;
use block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{Matrix, Pass, ProcessBlock};
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::sync::Arc;

/// FFT Block performs FFT on samples it accumulates.
///
/// On each time step a new sample is added to the buffer. When the buffer is full, FFT is performed on the samples.
/// The size of this buffer is set by the generic parameter `N`.
pub struct FftBlock<T: Float, const N: usize>
where
    OldBlockData: FromPass<Matrix<2, N, T>>,
{
    pub data: OldBlockData,
    /// Samples buffer that stores samples as we accumulate them.
    samples: [T; N],
    /// Index of the next sample to be added to the buffer.
    sample_index: usize,
    /// Output of the FFT block, only updated when the buffer is full.
    output: Matrix<2, N, T>,
    /// FFT planner used to create the FFT object.
    fft: Arc<dyn Fft<T>>,
}

impl<T: Float, const N: usize> Default for FftBlock<T, N> {
    fn default() -> Self {
        let mut planner = FftPlanner::<T>::new();
        let fft = planner.plan_fft_forward(N);
        Self {
            samples: [T::default(); N],
            sample_index: 0,
            output: Matrix::zeroed(),
            data: <OldBlockData as FromPass<Matrix<2, N, T>>>::from_pass(Matrix::zeroed().as_by()),
            fft,
        }
    }
}

impl<T: Float, const N: usize> ProcessBlock for FftBlock<T, N> {
    type Inputs = T;
    type Output = Matrix<2, N, T>;
    type Parameters = Parameters;

    fn process<'b>(
        &'b mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) -> pictorus_traits::PassBy<'b, Self::Output> {
        self.samples[self.sample_index] = inputs;

        if self.sample_index >= N - 1 {
            self.sample_index = 0;
            let mut data_buf: [Complex<T>; N] = [Complex::new(T::zero(), T::zero()); N];
            self.samples.iter().enumerate().for_each(|(i, &sample)| {
                data_buf[i].re = sample;
            });
            self.fft.process(&mut data_buf);
            for (i, buf_val) in data_buf.iter().enumerate().take(N) {
                self.output.data[i][0] = buf_val.re;
                self.output.data[i][1] = buf_val.im;
            }
            self.data = <OldBlockData as FromPass<Matrix<2, N, T>>>::from_pass(self.output.as_by());
        } else {
            self.sample_index += 1;
        }

        self.output.as_by()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Parameters;

impl Parameters {
    pub fn new() -> Parameters {
        Parameters
    }
}

#[cfg(test)]
mod tests {
    use core::f64;

    use super::*;
    use crate::testing::StubRuntime;
    use crate::SinewaveBlock;
    use approx::assert_relative_eq;
    use core::time::Duration;
    use pictorus_traits::GeneratorBlock;

    #[test]
    fn test_fft_block() {
        let mut runtime = StubRuntime::default();
        runtime.context.fundamental_timestep = Duration::from_secs_f32(0.1);

        // 2Hz sinewave, amplitude 5, with small bias
        let mut sinewave_2_hz: SinewaveBlock<f64> = SinewaveBlock::default();
        let sinewave_2_hz_parameters = <SinewaveBlock<f64> as GeneratorBlock>::Parameters::new(
            5.0,
            2.0 * f64::consts::TAU,
            0.0,
            1.2345,
        );

        // 3Hz sinewave, amplitude 10, with small phase shift
        let mut sinewave_3_hz: SinewaveBlock<f64> = SinewaveBlock::default();
        let sinewave_3_hz_parameters = <SinewaveBlock<f64> as GeneratorBlock>::Parameters::new(
            10.0,
            3.0 * f64::consts::TAU,
            0.5,
            0.0,
        );

        let mut fft_block: FftBlock<f64, 10> = FftBlock::default();
        let fft_parameters = Parameters::new();

        for _ in 0..100 {
            let output_2hz = sinewave_2_hz.generate(&sinewave_2_hz_parameters, &runtime.context);
            let output_3hz = sinewave_3_hz.generate(&sinewave_3_hz_parameters, &runtime.context);

            let combined = output_2hz + output_3hz;

            let _ = fft_block.process(&fft_parameters, &runtime.context, combined);
            runtime.tick();
        }

        let output_magnitudes = fft_block
            .output
            .data
            .map(|[re, im]| (re * re + im * im).sqrt());
        let expected_output = [
            12.345, // DC value (bias in the signal from 2hz signal)
            0.0,    // 1Hz
            25.0,   // 2Hz response twice as strong as DC bias
            50.0,   // 3Hz response twice as strong as 2Hz response
            0.0,    // 4Hz
            0.0,    // 5Hz
            0.0,    // 4Hz
            50.0,   // 3Hz
            25.0,   // 2Hz
            0.0,    // 1Hz
        ];

        for i in 0..10 {
            assert_relative_eq!(output_magnitudes[i], expected_output[i], epsilon = 0.01);
        }
    }
}
