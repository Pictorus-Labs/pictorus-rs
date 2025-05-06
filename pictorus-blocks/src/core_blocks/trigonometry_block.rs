use crate::traits::MatrixOps;
use block_data::{BlockData as OldBlockData, FromPass};
use num_traits::Float;
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock};

#[derive(strum::EnumString, PartialEq)]
pub enum TrigonometryFunction {
    Sine,
    Cosine,
    Tangent,
    ArcSine,
    ArcCosine,
    ArcTangent,
    SineHyperbolic,
    CosineHyperbolic,
    TangentHyperbolic,
    ArcSineHyperbolic,
    ArcCosineHyperbolic,
    ArcTangentHyperbolic,
}

pub struct Parameters {
    pub function: TrigonometryFunction,
}

impl Parameters {
    pub fn new(function: &str) -> Self {
        Self {
            function: function
                .parse()
                .expect("Failed to parse TrigonometryFunction"),
        }
    }
}

pub struct TrigonometryBlock<T> {
    pub data: OldBlockData,
    buffer: T,
}

impl<T> Default for TrigonometryBlock<T>
where
    T: Default + Pass,
    OldBlockData: FromPass<T>,
{
    fn default() -> Self {
        Self {
            data: <OldBlockData as FromPass<T>>::from_pass(T::default().as_by()),
            buffer: T::default(),
        }
    }
}

macro_rules! impl_trig_block {
    ($type:ty) => {
        impl ProcessBlock for TrigonometryBlock<$type> {
            type Inputs = $type;
            type Output = $type;
            type Parameters = Parameters;

            fn process(
                &mut self,
                parameters: &Self::Parameters,
                _context: &dyn pictorus_traits::Context,
                inputs: PassBy<'_, Self::Inputs>,
            ) -> PassBy<Self::Output> {
                let output = match parameters.function {
                    TrigonometryFunction::Sine => Float::sin(inputs),
                    TrigonometryFunction::Cosine => Float::cos(inputs),
                    TrigonometryFunction::Tangent => Float::tan(inputs),
                    TrigonometryFunction::ArcSine => Float::asin(inputs),
                    TrigonometryFunction::ArcCosine => Float::acos(inputs),
                    TrigonometryFunction::ArcTangent => Float::atan(inputs),
                    TrigonometryFunction::SineHyperbolic => Float::sinh(inputs),
                    TrigonometryFunction::CosineHyperbolic => Float::cosh(inputs),
                    TrigonometryFunction::TangentHyperbolic => Float::tanh(inputs),
                    TrigonometryFunction::ArcSineHyperbolic => Float::asinh(inputs),
                    TrigonometryFunction::ArcCosineHyperbolic => Float::acosh(inputs),
                    TrigonometryFunction::ArcTangentHyperbolic => Float::atanh(inputs),
                };
                self.buffer = output;
                self.data = OldBlockData::from_scalar(output.into());
                output
            }
        }

        impl<const ROWS: usize, const COLS: usize> ProcessBlock
            for TrigonometryBlock<Matrix<ROWS, COLS, $type>>
        where
            OldBlockData: FromPass<Matrix<ROWS, COLS, $type>>,
        {
            type Inputs = Matrix<ROWS, COLS, $type>;
            type Output = Matrix<ROWS, COLS, $type>;
            type Parameters = Parameters;

            fn process(
                &mut self,
                parameters: &Self::Parameters,
                _context: &dyn pictorus_traits::Context,
                inputs: PassBy<'_, Self::Inputs>,
            ) -> PassBy<Self::Output> {
                inputs.for_each(|input, c, r| {
                    let output = match parameters.function {
                        TrigonometryFunction::Sine => Float::sin(input),
                        TrigonometryFunction::Cosine => Float::cos(input),
                        TrigonometryFunction::Tangent => Float::tan(input),
                        TrigonometryFunction::ArcSine => Float::asin(input),
                        TrigonometryFunction::ArcCosine => Float::acos(input),
                        TrigonometryFunction::ArcTangent => Float::atan(input),
                        TrigonometryFunction::SineHyperbolic => Float::sinh(input),
                        TrigonometryFunction::CosineHyperbolic => Float::cosh(input),
                        TrigonometryFunction::TangentHyperbolic => Float::tanh(input),
                        TrigonometryFunction::ArcSineHyperbolic => Float::asinh(input),
                        TrigonometryFunction::ArcCosineHyperbolic => Float::acosh(input),
                        TrigonometryFunction::ArcTangentHyperbolic => Float::atanh(input),
                    };
                    self.buffer.data[c][r] = output;
                });
                self.data = OldBlockData::from_pass(&self.buffer);
                &self.buffer
            }
        }
    };
}

impl_trig_block!(f64);
impl_trig_block!(f32);

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use crate::testing::StubContext;
    use approx::assert_relative_eq;
    use core::f64::consts::PI;
    use rstest::rstest;

    #[rstest]
    #[case::sin_0("Sine", 0.0, 0.0)]
    #[case::sin_pi_2("Sine", PI / 2.0, 1.0)]
    #[case::cos_0("Cosine", 0.0, 1.0)]
    #[case::cos_pi_2("Cosine", PI / 2.0, 0.0)]
    #[case::tan_0("Tangent", 0.0, 0.0)]
    #[case::tan_pi_4("Tangent", PI / 4.0, 1.0)]
    #[case::asin_0("ArcSine", 0.0, 0.0)]
    #[case::asin_1("ArcSine", 1.0, PI / 2.0)]
    #[case::acos_1("ArcCosine", 1.0, 0.0)]
    #[case::acos_0("ArcCosine", 0.0, PI / 2.0)]
    #[case::atan_0("ArcTangent", 0.0, 0.0)]
    #[case::atan_1("ArcTangent", 1.0, PI / 4.0)]
    #[case::sinh_0("SineHyperbolic", 0.0, 0.0)]
    #[case::sinh_1("SineHyperbolic", 1.0, 1.17520)]
    #[case::cosh_0("CosineHyperbolic", 0.0, 1.0)]
    #[case::cosh_1("CosineHyperbolic", 1.0, 1.54308)]
    #[case::tanh_0("TangentHyperbolic", 0.0, 0.0)]
    #[case::tanh_1("TangentHyperbolic", 1.0, 0.76159)]
    #[case::asinh_0("ArcSineHyperbolic", 0.0, 0.0)]
    #[case::asinh_1_17520("ArcSineHyperbolic", 1.17520, 1.0)]
    #[case::acosh_1("ArcCosineHyperbolic", 1.0, 0.0)]
    #[case::acosh_1_54308("ArcCosineHyperbolic", 1.54308, 1.0)]
    #[case::atanh_0("ArcTangentHyperbolic", 0.0, 0.0)]
    #[case::atanh_0_76159("ArcTangentHyperbolic", 0.76159, 1.0)]
    fn test_trig_functions(
        #[case] function: &'static str,
        #[case] input: f64,
        #[case] expected: f64,
    ) {
        let c = StubContext::default();
        let mut block = TrigonometryBlock::<f64>::default();
        let p = Parameters::new(function);

        let output = block.process(&p, &c, input);
        assert_relative_eq!(output, expected, max_relative = 0.00001);
        assert_relative_eq!(block.data.scalar(), expected, max_relative = 0.00001);
    }

    #[test]
    fn test_trigonometry_block_vectorized() {
        let c = StubContext::default();
        let mut sine_block = TrigonometryBlock::<Matrix<1, 2, f64>>::default();
        let p = Parameters::new("Sine");
        let inputs = Matrix {
            data: [[0.0], [PI / 2.0]],
        };

        let output = sine_block.process(&p, &c, &inputs);
        assert_relative_eq!(
            output.data.as_flattened(),
            [[0.0], [1.0]].as_flattened(),
            max_relative = 0.00001
        );
    }
}
