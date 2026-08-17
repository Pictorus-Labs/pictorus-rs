use crate::traits::Scalar;
use core::ops::Mul;
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock};

/// Multiplies the input by a gain factor.
pub struct GainBlock<T>
where
    T: Apply,
{
    buffer: T,
}

impl<T> Default for GainBlock<T>
where
    T: Apply,
{
    fn default() -> Self {
        Self {
            buffer: T::default(),
        }
    }
}

impl<T> ProcessBlock for GainBlock<T>
where
    T: Apply,
{
    type Inputs = T;
    type Output = T;
    type Parameters = Parameters<T::Gain>;

    fn process(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        input: PassBy<Self::Inputs>,
    ) -> PassBy<'_, Self::Output> {
        T::apply(&mut self.buffer, input, parameters.gain)
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        self.buffer.as_by()
    }
}

pub trait Apply: Pass + Default {
    /// The scalar type of the gain parameter. Matches the element type for
    /// matrix signals and the signal type itself for scalar signals.
    type Gain: Scalar;

    fn apply<'s>(store: &'s mut Self, input: PassBy<Self>, gain: Self::Gain) -> PassBy<'s, Self>;
}

impl<S> Apply for S
where
    S: Scalar + Mul<Output = S>,
{
    type Gain = S;

    fn apply<'s>(store: &'s mut Self, input: PassBy<Self>, gain: S) -> PassBy<'s, Self> {
        let output = input * gain;
        *store = output;
        output
    }
}

impl<const NROWS: usize, const NCOLS: usize, S> Apply for Matrix<NROWS, NCOLS, S>
where
    S: Scalar + Apply<Gain = S>,
{
    type Gain = S;

    fn apply<'s>(store: &'s mut Self, input: PassBy<Self>, gain: S) -> PassBy<'s, Self> {
        for (elem, &val) in store
            .data
            .as_flattened_mut()
            .iter_mut()
            .zip(input.data.as_flattened().iter())
        {
            S::apply(elem, val, gain);
        }
        store
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
    use paste::paste;

    #[test]
    fn test_gain_default_buffer_no_panic() {
        let block = GainBlock::<f64>::default();
        assert_eq!(block.buffer(), 0.0);

        let block = GainBlock::<Matrix<2, 2, f64>>::default();
        assert_eq!(block.buffer(), &Matrix::<2, 2, f64>::zeroed());
    }

    macro_rules! test_gain_block {
        // Convenience call to generate a call to the main macro for every type in the list
        ($type:ty, $($other_types:ty),*) => {
            test_gain_block!($type);
            test_gain_block!($($other_types),*);
        };
        ($type:ty) => {
            paste! {
                #[test]
                fn [<test_gain_scalar_ $type>]() {
                    let mut block = GainBlock::<$type>::default();
                    let context = StubContext::default();
                    let input = 3 as $type;
                    let parameters = Parameters::new(2 as $type);
                    let output = block.process(&parameters, &context, input);
                    assert_eq!(output, 6 as $type);
                    assert_eq!(block.buffer(), output);
                }

                #[test]
                fn [<test_gain_matrix_ $type>]() {
                    let mut block = GainBlock::<Matrix<2, 2, $type>>::default();
                    let context = StubContext::default();
                    let input = Matrix {
                        data: [[1 as $type, 2 as $type], [3 as $type, 4 as $type]],
                    };
                    let parameters = Parameters::new(2 as $type);
                    let output = block.process(&parameters, &context, &input);
                    let expected = [
                        [2 as $type, 4 as $type],
                        [6 as $type, 8 as $type],
                    ];
                    assert_eq!(output.data, expected);
                    assert_eq!(block.buffer().data, expected);
                }
            }
        };
    }

    test_gain_block!(f32, f64, i8, i16, i32, i64, u8, u16, u32, u64);

    #[test]
    fn test_gain_negative() {
        let mut block = GainBlock::<i32>::default();
        let context = StubContext::default();
        let parameters = Parameters::new(-3);
        let output = block.process(&parameters, &context, 7);
        assert_eq!(output, -21);
        assert_eq!(block.buffer(), -21);
    }

    #[test]
    #[should_panic]
    fn overflow_panics() {
        // Native integer multiplication overflow panics in debug builds (wraps in release).
        let mut block = GainBlock::<u8>::default();
        let parameters = Parameters::new(2u8);
        let context = StubContext::default();

        let output = block.process(&parameters, &context, 128u8);
        assert_eq!(output, 0);
    }
}
