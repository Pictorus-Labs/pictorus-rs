use core::ops::Mul;

use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock, Scalar};

/// Multiplies the input by a gain factor.
pub struct GainBlock<G, T>
where
    G: Scalar,
    T: Apply<G>,
{
    buffer: Option<T::Output>,
}

impl<G, T> Default for GainBlock<G, T>
where
    G: Scalar,
    T: Apply<G>,
{
    fn default() -> Self {
        Self { buffer: None }
    }
}

impl<G, T> ProcessBlock for GainBlock<G, T>
where
    G: Scalar,
    T: Apply<G>,
{
    type Inputs = T;
    type Output = T::Output;
    type Parameters = Parameters<G>;

    fn process(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        input: PassBy<Self::Inputs>,
    ) -> PassBy<'_, Self::Output> {
        let output = T::apply(&mut self.buffer, input, parameters.gain);
        output
    }
}

pub trait Apply<G: Scalar>: Pass {
    type Output: Pass + Default;

    fn apply<'s>(
        store: &'s mut Option<Self::Output>,
        input: PassBy<Self>,
        gain: G,
    ) -> PassBy<'s, Self::Output>;
}

impl<G> Apply<G> for f64
where
    G: Scalar,
    f64: Mul<G>,
    <f64 as Mul<G>>::Output: Scalar,
{
    type Output = <f64 as Mul<G>>::Output;
    fn apply<'s>(
        store: &'s mut Option<Self::Output>,
        input: PassBy<Self>,
        gain: G,
    ) -> PassBy<'s, Self::Output> {
        let output = input * gain;
        *store = Some(output);
        output.as_by()
    }
}

impl<G> Apply<G> for f32
where
    G: Scalar,
    f32: Mul<G>,
    <f32 as Mul<G>>::Output: Scalar,
{
    type Output = <f32 as Mul<G>>::Output;
    fn apply<'s>(
        store: &'s mut Option<Self::Output>,
        input: PassBy<Self>,
        gain: G,
    ) -> PassBy<'s, Self::Output> {
        let output = input * gain;
        *store = Some(output);
        output.as_by()
    }
}

impl<const NROWS: usize, const NCOLS: usize, G, T> Apply<G> for Matrix<NROWS, NCOLS, T>
where
    T: Scalar,
    G: Scalar,
    T: Mul<G, Output = T>,
{
    type Output = Matrix<NROWS, NCOLS, T>;

    fn apply<'s>(
        store: &'s mut Option<Self::Output>,
        input: PassBy<Self>,
        gain: G,
    ) -> PassBy<'s, Self::Output> {
        let output = store.insert(Matrix::zeroed());
        output
            .data
            .as_flattened_mut()
            .iter_mut()
            .enumerate()
            .for_each(|(i, lhs)| {
                let input_val = input.data.as_flattened()[i];
                *lhs = input_val * gain;
            });
        output
    }
}

pub struct Parameters<G: Scalar> {
    pub gain: G,
}

impl<G: Scalar> Parameters<G> {
    pub fn new(gain: G) -> Self {
        Self { gain }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;

    #[test]
    fn test_gain_scalar() {
        let mut block = GainBlock::<f64, f64>::default();
        let context = StubContext::default();
        let input = 1.0;
        let parameters = Parameters::new(2.0);
        let output = block.process(&parameters, &context, input);
        assert_eq!(output, 2.0);
    }

    #[test]
    fn test_gain_matrix() {
        let mut block = GainBlock::<f64, Matrix<2, 2, f64>>::default();
        let context = StubContext::default();
        let input = Matrix {
            data: [[1.0, 2.0], [3.0, 4.0]],
        };
        let parameters = Parameters::new(2.0);
        let output = block.process(&parameters, &context, &input);
        assert_eq!(output.data, [[2.0, 4.0], [6.0, 8.0]]);
    }
}
