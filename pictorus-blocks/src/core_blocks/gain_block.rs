use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock, Promote, Promotion, Scalar};

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
    G: Promote<f64> + Scalar,
{
    type Output = Promotion<G, f64>;
    fn apply<'s>(
        store: &'s mut Option<Self::Output>,
        input: PassBy<Self>,
        gain: G,
    ) -> PassBy<'s, Self::Output> {
        let output =
            <G as Promote<f64>>::promote_left(gain) * <G as Promote<f64>>::promote_right(input);
        *store = Some(output);
        output
    }
}

impl<T> Apply<T> for f32
where
    T: Promote<f32> + Scalar,
{
    type Output = Promotion<T, f32>;
    fn apply<'s>(
        store: &'s mut Option<Self::Output>,
        input: PassBy<Self>,
        gain: T,
    ) -> PassBy<'s, Self::Output> {
        let output =
            <T as Promote<f32>>::promote_left(gain) * <T as Promote<f32>>::promote_right(input);
        *store = Some(output);
        output
    }
}

impl<const NROWS: usize, const NCOLS: usize, G, T> Apply<G> for Matrix<NROWS, NCOLS, T>
where
    T: Scalar,
    G: Promote<T>,
    T: Promote<G>,
{
    type Output = Matrix<NROWS, NCOLS, Promotion<G, T>>;

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
                *lhs = <G as Promote<T>>::promote_right(input.data.as_flattened()[i])
                    * <G as Promote<T>>::promote_left(gain);
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
