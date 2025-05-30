use pictorus_block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{HasIc, Matrix, Pass, PassBy, ProcessBlock};

use super::derivative_block::Parameters as DerivativeParameters;
use super::integral_block::{
    Apply as IntegralApply, IntgeralMethod, Parameters as IntegralParameters,
};
use crate::traits::{Float, MatrixOps};
use crate::{DerivativeBlock, IntegralBlock, Scalar};

/// Performs PID (Proportional, Integral, Derivative) control against an error signal.
///
/// The input signal can either be a scalar or a matrix.
/// In the case of a matrix, the PID control is applied element-wise.
///
/// This block also accepts a second reset input, which can be used to reset the
/// integrator.
pub struct PidBlock<T: ComponentOps, R: Scalar, const ND_SAMPLES: usize>
where
    OldBlockData: FromPass<T>,
    (T, R): IntegralApply<Output = T>,
{
    pub data: OldBlockData,
    buffer: T,
    integrator: IntegralBlock<(T, R)>,
    derivative: DerivativeBlock<T, ND_SAMPLES>,
}

impl<T: ComponentOps, R: Scalar, const ND_SAMPLES: usize> Default for PidBlock<T, R, ND_SAMPLES>
where
    OldBlockData: FromPass<T>,
    (T, R): IntegralApply<Output = T>,
{
    fn default() -> Self {
        Self {
            data: <OldBlockData as FromPass<T>>::from_pass(T::default().as_by()),
            buffer: T::default(),
            integrator: IntegralBlock::default(),
            derivative: DerivativeBlock::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
/// Parameters for the PID block
pub struct Parameters<T: IntegralApply> {
    /// Initial condition for the integrator
    ic: T::Output,
    /// Proportional gain
    kp: T::Float,
    /// Integral gain
    ki: T::Float,
    /// Derivative gain
    kd: T::Float,
    /// Maximum value for the integrator
    i_max: T::Float,
}

impl<T: IntegralApply> Parameters<T> {
    pub fn new(ic: T::Output, kp: T::Float, ki: T::Float, kd: T::Float, i_max: T::Float) -> Self {
        Self {
            ic,
            kp,
            ki,
            kd,
            i_max,
        }
    }
}

impl<T: ComponentOps, R: Scalar, const ND_SAMPLES: usize> PidBlock<T, R, ND_SAMPLES>
where
    OldBlockData: FromPass<T>,
    (T, R): IntegralApply<Output = T, Float = T::Float>,
{
    fn integrator_params(parameters: &Parameters<(T, R)>) -> IntegralParameters<(T, R)> {
        IntegralParameters {
            clamp_limit: parameters.i_max,
            ic: parameters.ic,
            method: IntgeralMethod::Rectangle,
        }
    }

    fn derivative_params(parameters: &Parameters<(T, R)>) -> DerivativeParameters<T> {
        DerivativeParameters { ic: parameters.ic }
    }
}
impl<T: ComponentOps, R: Scalar, const ND_SAMPLES: usize> ProcessBlock
    for PidBlock<T, R, ND_SAMPLES>
where
    OldBlockData: FromPass<T>,
    DerivativeBlock<T, ND_SAMPLES>:
        ProcessBlock<Output = T, Inputs = T, Parameters = DerivativeParameters<T>>,
    IntegralBlock<(T, R)>:
        ProcessBlock<Output = T, Inputs = (T, R), Parameters = IntegralParameters<(T, R)>>,
    (T, R): IntegralApply<Output = T, Float = T::Float> + for<'a> Pass<By<'a> = (PassBy<'a, T>, R)>,
{
    type Inputs = (T, R);
    type Output = T;
    type Parameters = Parameters<(T, R)>;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        context: &dyn pictorus_traits::Context,
        inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) -> pictorus_traits::PassBy<'b, Self::Output> {
        let integrator_params = Self::integrator_params(parameters);
        // Run integrator
        let (sample, reset): (PassBy<'_, T>, R) = inputs;
        let i_sample = T::component_mul(sample, parameters.ki);
        let i = ProcessBlock::process(
            &mut self.integrator,
            &integrator_params,
            context,
            (i_sample.as_by(), reset),
        );

        // Run derivative
        let derivative_params = Self::derivative_params(parameters);

        let d_res =
            ProcessBlock::process(&mut self.derivative, &derivative_params, context, sample);

        // Add them all up!
        let p = T::component_mul(sample, parameters.kp);
        let d = T::component_mul(d_res, parameters.kd);
        self.buffer = T::component_add(p.as_by(), i, d.as_by());

        self.data = OldBlockData::from_pass(self.buffer.as_by());
        self.buffer.as_by()
    }
}

// TODO: This is currently only implemented for f64 types. The IntegralBlock
// and DerivativeBlock are implemented using very different approaches: integral
// block uses a trait-based approach, and derivative block uses macros. I think if we
// consolidated our approach for these 3 blocks we could make this simpler and more generic.
// Ideally we could just have a blanket impl for <const ND_SAMPLES: usize, T: ComponentOps>.
impl<const ND_SAMPLES: usize, R: Scalar> HasIc for PidBlock<f64, R, ND_SAMPLES> {
    fn new(parameters: &Self::Parameters) -> Self {
        let integrator_params = Self::integrator_params(parameters);
        let derivative_params = Self::derivative_params(parameters);
        Self {
            data: OldBlockData::from_scalar(parameters.ic),
            buffer: 0.0,
            integrator: IntegralBlock::new(&integrator_params),
            derivative: DerivativeBlock::new(&derivative_params),
        }
    }
}

impl<const ND_SAMPLES: usize, const NROWS: usize, const NCOLS: usize, R: Scalar> HasIc
    for PidBlock<Matrix<NROWS, NCOLS, f64>, R, ND_SAMPLES>
where
    OldBlockData: FromPass<Matrix<NROWS, NCOLS, f64>>,
{
    fn new(parameters: &Self::Parameters) -> Self {
        let integrator_params = Self::integrator_params(parameters);
        let derivative_params = Self::derivative_params(parameters);
        Self {
            data: OldBlockData::from_pass(parameters.ic.as_by()),
            buffer: Matrix::default(),
            integrator: IntegralBlock::new(&integrator_params),
            derivative: DerivativeBlock::new(&derivative_params),
        }
    }
}

// It would be nice to have these types of common operators defined somewhere reusable
// I.e. mixed scalar/matrix addition, multiplication, etc. This would reduce a lot
// of repetition in block implementations
pub trait ComponentOps: Pass + Default + Copy {
    type Float: Float;
    fn component_mul(lhs: PassBy<Self>, rhs: Self::Float) -> Self;
    fn component_add(v1: PassBy<Self>, v2: PassBy<Self>, v3: PassBy<Self>) -> Self;
}

impl<F: Float> ComponentOps for F {
    type Float = F;
    fn component_mul(lhs: F, rhs: F) -> Self {
        lhs * rhs
    }

    fn component_add(v1: F, v2: F, v3: F) -> Self {
        v1 + v2 + v3
    }
}

impl<const NROWS: usize, const NCOLS: usize, F: Float> ComponentOps for Matrix<NROWS, NCOLS, F> {
    type Float = F;
    fn component_mul(lhs: PassBy<Self>, rhs: Self::Float) -> Self {
        let mut res = Self::default();
        lhs.for_each(|v, c, r| {
            res.data[c][r] = v * rhs;
        });
        res
    }

    fn component_add(v1: PassBy<Self>, v2: PassBy<Self>, v3: PassBy<Self>) -> Self {
        let mut res = Self::default();
        v1.for_each(|v, c, r| {
            res.data[c][r] = v + v2.data[c][r] + v3.data[c][r];
        });
        res
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use super::*;
    use crate::testing::{StubContext, StubRuntime};
    use approx::assert_relative_eq;

    #[test]
    fn test_p_scalar() {
        let mut runtime = StubRuntime::new(StubContext::new(
            Duration::ZERO,
            None,
            Duration::from_secs(1),
        ));
        let params = Parameters::new(0.0, 2.0, 0.0, 0.0, 0.0);
        let mut p_block = PidBlock::<f64, bool, 2>::default();

        // Output should just be double the input
        let res = p_block.process(&params, &runtime.context(), (1.0, false));
        assert_eq!(res, 2.0);
        assert_eq!(p_block.data.scalar(), 2.0);
        runtime.tick();

        let res = p_block.process(&params, &runtime.context(), (-2.0, false));
        assert_eq!(res, -4.0);
        assert_eq!(p_block.data.scalar(), -4.0);
    }

    #[test]
    fn test_i_scalar() {
        let mut runtime = StubRuntime::new(StubContext::new(
            Duration::ZERO,
            None,
            Duration::from_secs(1),
        ));

        let params = Parameters::new(0.0, 0.0, 3.0, 0.0, 10.0);
        let mut i_block = PidBlock::<f64, bool, 2>::default();

        let res = i_block.process(&params, &runtime.context(), (0.0, false));
        assert_eq!(res, 0.0);
        assert_eq!(i_block.data.scalar(), 0.0);
        runtime.tick();

        i_block.process(&params, &runtime.context(), (0.0, false));
        let res = i_block.process(&params, &runtime.context(), (1.0, false));
        assert_relative_eq!(res, 3.0, max_relative = 0.01);
        assert_relative_eq!(i_block.data.scalar(), 3.0, max_relative = 0.01);
        runtime.tick();

        // Make sure it actually integrates
        let res = i_block.process(&params, &runtime.context(), (1.0, false));
        assert_relative_eq!(res, 6.0, max_relative = 0.01);
        assert_relative_eq!(i_block.data.scalar(), 6.0, max_relative = 0.01);
        runtime.tick();

        // Check saturation
        let res = i_block.process(&params, &runtime.context(), (100.0, false));
        assert_relative_eq!(res, 10.0, max_relative = 0.01);
        assert_relative_eq!(i_block.data.scalar(), 10.0, max_relative = 0.01);
        runtime.tick();

        // Test reset
        let res = i_block.process(&params, &runtime.context(), (1.0, true));
        assert_relative_eq!(res, 0.0, max_relative = 0.01);
        assert_relative_eq!(i_block.data.scalar(), 0.0, max_relative = 0.01);
    }

    #[test]
    fn test_d_scalar() {
        let mut runtime = StubRuntime::new(StubContext::new(
            Duration::ZERO,
            None,
            Duration::from_secs_f64(0.5),
        ));

        let params = Parameters::new(0.0, 0.0, 0.0, 1.0, 0.0);
        let mut d_block = PidBlock::<f64, bool, 2>::default();
        d_block.process(&params, &runtime.context(), (0.0, false)); // Need at least 2 samples to estimate derivative
        runtime.tick();

        let res = d_block.process(&params, &runtime.context(), (100.0, false));
        assert_relative_eq!(res, 200.0, max_relative = 0.01);
        assert_relative_eq!(d_block.data.scalar(), 200.0, max_relative = 0.01);
    }
    #[test]
    fn test_pid_scalar() {
        let mut runtime = StubRuntime::new(StubContext::new(
            Duration::ZERO,
            None,
            Duration::from_secs_f64(1.0),
        ));
        let params = Parameters::new(0.0, 1.0, 2.0, 3.0, 10.0);
        let mut block = PidBlock::<f64, bool, 2>::default();

        let res = block.process(&params, &runtime.context(), (0.0, false));
        assert_relative_eq!(res, 0.0, max_relative = 0.01);
        runtime.tick();

        // p: 2, i: 4, d: 6
        let res = block.process(&params, &runtime.context(), (2.0, false));
        assert_relative_eq!(res, 12.0, max_relative = 0.01);
        assert_relative_eq!(block.data.scalar(), 12.0, max_relative = 0.01);
    }

    #[test]
    fn test_pid_scalar_with_ic() {
        let mut runtime = StubRuntime::new(StubContext::new(
            Duration::ZERO,
            None,
            Duration::from_secs_f64(1.0),
        ));
        let params = Parameters::new(5.0, 1.0, 2.0, 3.0, 10.0);
        let mut block = PidBlock::<f64, bool, 2>::default();

        let res = block.process(&params, &runtime.context(), (0.0, false));
        assert_relative_eq!(res, 5.0, max_relative = 0.01);
        runtime.tick();

        // p: 2, i: 5 + 4 = 9, d: 6
        let res = block.process(&params, &runtime.context(), (2.0, false));
        assert_relative_eq!(res, 17.0, max_relative = 0.01);
        assert_relative_eq!(block.data.scalar(), 17.0, max_relative = 0.01);
    }

    #[test]
    fn test_p_matrix() {
        let mut runtime = StubRuntime::new(StubContext::new(
            Duration::ZERO,
            None,
            Duration::from_secs_f64(1.0),
        ));
        let params = Parameters::new(Matrix::zeroed(), 2.0, 0.0, 0.0, 0.0);
        let mut p_block = PidBlock::<Matrix<2, 2, f64>, bool, 2>::default();

        let input = Matrix {
            data: [[1.0, 2.0], [3.0, 4.0]],
        };
        let res = p_block.process(&params, &runtime.context(), (&input, false));
        let expected = Matrix {
            data: [[2.0, 4.0], [6.0, 8.0]],
        };
        assert_eq!(res, &expected);
        assert_eq!(
            p_block.data.get_data().as_slice(),
            expected.data.as_flattened()
        );
        runtime.tick();

        let input = Matrix {
            data: [[-2.0, -3.0], [-4.0, -5.0]],
        };
        let res = p_block.process(&params, &runtime.context(), (&input, false));
        let expected = Matrix {
            data: [[-4.0, -6.0], [-8.0, -10.0]],
        };
        assert_eq!(res, &expected);
        assert_eq!(
            p_block.data.get_data().as_slice(),
            expected.data.as_flattened()
        );
    }

    #[test]
    fn test_i_matrix() {
        let mut runtime = StubRuntime::new(StubContext::new(
            Duration::ZERO,
            None,
            Duration::from_secs_f64(1.0),
        ));

        let params = Parameters::new(Matrix::zeroed(), 0.0, 3.0, 0.0, 10.0);
        let mut i_block = PidBlock::<Matrix<2, 2, f64>, bool, 2>::default();

        let input = Matrix {
            data: [[0.0, 0.0], [0.0, 0.0]],
        };
        let res = i_block.process(&params, &runtime.context(), (&input, false));
        let expected = Matrix {
            data: [[0.0, 0.0], [0.0, 0.0]],
        };
        assert_eq!(res, &expected);
        assert_eq!(
            i_block.data.get_data().as_slice(),
            expected.data.as_flattened()
        );
        runtime.tick();

        let input = Matrix {
            data: [[0.0, 0.0], [1.0, 1.0]],
        };
        let res = i_block.process(&params, &runtime.context(), (&input, false));
        let expected = Matrix {
            data: [[0.0, 0.0], [3.0, 3.0]],
        };
        assert_eq!(res, &expected);
        assert_eq!(
            i_block.data.get_data().as_slice(),
            expected.data.as_flattened()
        );
        runtime.tick();

        // Make sure it actually integrates
        let input = Matrix {
            data: [[0.0, 0.0], [1.0, 1.0]],
        };
        let res = i_block.process(&params, &runtime.context(), (&input, false));
        let expected = Matrix {
            data: [[0.0, 0.0], [6.0, 6.0]],
        };
        assert_eq!(res, &expected);
        assert_eq!(
            i_block.data.get_data().as_slice(),
            expected.data.as_flattened()
        );
        runtime.tick();

        // Check saturation
        let input = Matrix {
            data: [[0.0, 0.0], [100.0, 100.0]],
        };
        let res = i_block.process(&params, &runtime.context(), (&input, false));
        let expected = Matrix {
            data: [[0.0, 0.0], [10.0, 10.0]],
        };
        assert_eq!(res, &expected);
        assert_eq!(
            i_block.data.get_data().as_slice(),
            expected.data.as_flattened()
        );
        runtime.tick();
    }

    #[test]
    fn test_d_matrix() {
        let mut runtime = StubRuntime::new(StubContext::new(
            Duration::ZERO,
            None,
            Duration::from_secs_f64(0.5),
        ));

        let params = Parameters::new(Matrix::zeroed(), 0.0, 0.0, 1.0, 0.0);
        let mut d_block = PidBlock::<Matrix<2, 2, f64>, bool, 2>::default();
        d_block.process(&params, &runtime.context(), (&Matrix::zeroed(), false)); // Need at least 2 samples to estimate derivative
        runtime.tick();

        let input = Matrix {
            data: [[100.0, 200.0], [300.0, 400.0]],
        };
        let res = d_block.process(&params, &runtime.context(), (&input, false));
        let expected = Matrix {
            data: [[200.0, 400.0], [600.0, 800.0]],
        };
        assert_eq!(res, &expected);
        assert_eq!(
            d_block.data.get_data().as_slice(),
            expected.data.as_flattened()
        );
    }

    #[test]
    fn test_pid_matrix() {
        let mut runtime = StubRuntime::new(StubContext::new(
            Duration::ZERO,
            None,
            Duration::from_secs_f64(1.0),
        ));
        let params = Parameters::new(Matrix::zeroed(), 1.0, 2.0, 3.0, 10.0);
        let mut block = PidBlock::<Matrix<2, 2, f64>, bool, 2>::default();

        let input = Matrix {
            data: [[0.0, 0.0], [0.0, 0.0]],
        };
        let res = block.process(&params, &runtime.context(), (&input, false));
        let expected = Matrix {
            data: [[0.0, 0.0], [0.0, 0.0]],
        };
        assert_eq!(res, &expected);
        assert_eq!(
            block.data.get_data().as_slice(),
            expected.data.as_flattened()
        );
        runtime.tick();

        let input = Matrix {
            data: [[1.0, 2.0], [3.0, 4.0]],
        };
        let res = block.process(&params, &runtime.context(), (&input, false));
        let expected = Matrix {
            data: [[6.0, 12.0], [18.0, 24.0]],
        };
        assert_eq!(res, &expected);
        assert_eq!(
            block.data.get_data().as_slice(),
            expected.data.as_flattened()
        );
    }

    #[test]
    fn test_pid_matrix_with_ic() {
        let mut runtime = StubRuntime::new(StubContext::new(
            Duration::ZERO,
            None,
            Duration::from_secs_f64(1.0),
        ));
        let ic = Matrix {
            data: [[4.0, 5.0], [6.0, 7.0]],
        };
        let params = Parameters::new(ic, 1.0, 2.0, 3.0, 10.0);
        let mut block = PidBlock::<Matrix<2, 2, f64>, bool, 2>::default();

        let input = Matrix {
            data: [[0.0, 0.0], [0.0, 0.0]],
        };
        let res = block.process(&params, &runtime.context(), (&input, false));
        let expected = Matrix {
            data: [[4.0, 5.0], [6.0, 7.0]],
        };
        assert_eq!(res, &expected);
        assert_eq!(
            block.data.get_data().as_slice(),
            expected.data.as_flattened()
        );
        runtime.tick();

        let input = Matrix {
            data: [[1.0, 2.0], [3.0, 4.0]],
        };
        let res = block.process(&params, &runtime.context(), (&input, false));
        // The I components of [1][0] and [1][1] are saturated at 10, so they are
        // lower than expected offset from the IC
        let expected = Matrix {
            data: [[10.0, 17.0], [22.0, 26.0]],
        };
        assert_eq!(res, &expected);
        assert_eq!(
            block.data.get_data().as_slice(),
            expected.data.as_flattened()
        );
    }
}
