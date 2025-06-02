use core::ops::MulAssign;

use crate::nalgebra_interop::MatrixExt;
use crate::traits::{MatrixOps, Scalar};
use nalgebra::ClosedDivAssign;
use num_traits::Float;
use pictorus_block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock};

pub struct Parameters<I: Scalar + Float> {
    /// The scalar interval to quantize to
    interval: I,
}

impl<I: Scalar + Float> Parameters<I> {
    pub fn new(interval: I) -> Self {
        Parameters { interval }
    }
}

/// Quantizes the input to the nearest integer multiple of the provided interval.
///
/// For example, if the interval is 0.5, the input 0.51 will be quantized to 0.5
/// If the interval is 0.5, the input 0.75 will be quantized to 1.0
/// For matrices, the process is applied element-wise
pub struct QuantizeBlock<I, T>
where
    I: Scalar + Float,
    T: Apply<I>,
    OldBlockData: FromPass<T::Output>,
{
    pub data: OldBlockData,
    buffer: Option<T::Output>,
}

impl<I, T> Default for QuantizeBlock<I, T>
where
    I: Scalar + Float,
    T: Apply<I>,
    OldBlockData: FromPass<T::Output>,
{
    fn default() -> Self {
        Self {
            data: <OldBlockData as FromPass<T::Output>>::from_pass(T::Output::default().as_by()),
            buffer: None,
        }
    }
}

impl<I, T> ProcessBlock for QuantizeBlock<I, T>
where
    I: Scalar + Float,
    T: Apply<I>,
    OldBlockData: FromPass<T::Output>,
{
    type Parameters = Parameters<I>;
    type Inputs = T;
    type Output = T::Output;

    fn process(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<Self::Output> {
        let res = T::apply(inputs, parameters.interval, &mut self.buffer);
        self.data = OldBlockData::from_pass(res);
        res
    }
}

pub trait Apply<I: Scalar + Float>: Pass + Default {
    type Output: Pass + Default;

    fn apply<'a>(
        input: PassBy<Self>,
        interval: I,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output>;
}

impl<I: Scalar + Float> Apply<I> for I {
    type Output = I;

    fn apply<'a>(
        input: PassBy<Self>,
        interval: I,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let input_divided_interval = input / interval;
        let rounded = input_divided_interval.round();
        let res = rounded * interval;
        *dest = Some(res);
        res
    }
}

impl<const R: usize, const C: usize, I: Scalar + Float + ClosedDivAssign + MulAssign> Apply<I>
    for Matrix<R, C, I>
{
    type Output = Matrix<R, C, I>;

    fn apply<'a>(
        input: PassBy<Self>,
        interval: I,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let interval_matrix = Self::from_element(interval);
        let input_divided_interval = input.as_view().component_div(&interval_matrix.as_view());
        let rounded = input_divided_interval.map(Float::round);
        let res = rounded * interval;
        let res = Self::from_view(&res.as_view());
        *dest = Some(res);
        dest.as_ref().unwrap().as_by()
    }
}

#[cfg(test)]
mod tests {
    use std::vec::Vec;

    use crate::testing::StubContext;
    use paste::paste;

    use super::*;

    macro_rules! test_quantize_block {
        ($type:ty) => {
            paste! {
                #[test]
                fn [<test_quantize_block_scalar _$type>]() {
                    let context = StubContext::default();
                    let params = Parameters::new(0.5);
                    let mut block = QuantizeBlock::<$type, $type>::default();
                    let input = 0.51;
                    let res = block.process(&params, &context, input);

                    assert_eq!(res, 0.5);
                    assert_eq!(block.data.scalar(), 0.5);
                }

                #[test]
                fn [<test_quantize_block_matrix _$type>]() {
                    let context = StubContext::default();
                    let params = Parameters::new(0.5);
                    let mut block = QuantizeBlock::<$type, Matrix<4, 1, $type>>::default();
                    let input = Matrix {
                        data: [[0.24, 0.25, 0.51, 0.75]],
                    };
                    let expected = Matrix {
                        data: [[0.0, 0.5, 0.5, 1.0]],
                    };
                    let res = block.process(&params, &context, &input);

                    assert_eq!(res.data, expected.data);
                    assert_eq!(
                        block.data.get_data().as_slice(),
                        expected
                            .data
                            .as_flattened()
                            .iter()
                            .map(|x| *x as f64)
                            .collect::<Vec<f64>>()
                    );
                }
            }
        };
    }

    test_quantize_block!(f32);
    test_quantize_block!(f64);
}
