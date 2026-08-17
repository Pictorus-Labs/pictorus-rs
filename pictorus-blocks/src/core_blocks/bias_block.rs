use crate::traits::Scalar;
use core::ops::Add;
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock};

/// Outputs the input data with an added bias (offset).
pub struct BiasBlock<T>
where
    T: Apply,
{
    buffer: T,
}

impl<T> Default for BiasBlock<T>
where
    T: Apply,
{
    fn default() -> Self {
        Self {
            buffer: T::default(),
        }
    }
}

impl<T> ProcessBlock for BiasBlock<T>
where
    T: Apply,
{
    type Inputs = T;
    type Output = T;
    type Parameters = Parameters<T::Offset>;

    fn process(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        input: PassBy<Self::Inputs>,
    ) -> PassBy<'_, Self::Output> {
        T::apply(&mut self.buffer, input, parameters.offset)
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        self.buffer.as_by()
    }
}

pub trait Apply: Pass + Default {
    /// The scalar type of the offset parameter. Matches the element type for
    /// matrix signals and the signal type itself for scalar signals.
    type Offset: Scalar;

    fn apply<'s>(
        store: &'s mut Self,
        input: PassBy<Self>,
        offset: Self::Offset,
    ) -> PassBy<'s, Self>;
}

impl<S> Apply for S
where
    S: Scalar + Add<Output = S>,
{
    type Offset = S;

    fn apply<'s>(store: &'s mut Self, input: PassBy<Self>, offset: S) -> PassBy<'s, Self> {
        let output = input + offset;
        *store = output;
        output
    }
}

impl<const NROWS: usize, const NCOLS: usize, S> Apply for Matrix<NROWS, NCOLS, S>
where
    S: Scalar + Apply<Offset = S>,
{
    type Offset = S;

    fn apply<'s>(store: &'s mut Self, input: PassBy<Self>, offset: S) -> PassBy<'s, Self> {
        for (elem, &val) in store
            .data
            .as_flattened_mut()
            .iter_mut()
            .zip(input.data.as_flattened().iter())
        {
            S::apply(elem, val, offset);
        }
        store
    }
}

pub struct Parameters<B>
where
    B: Scalar,
{
    pub offset: B,
}

impl<B> Parameters<B>
where
    B: Scalar,
{
    pub fn new(offset: B) -> Self {
        Self { offset }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use approx::assert_relative_eq;
    use paste::paste;

    #[test]
    fn test_bias_default_buffer_no_panic() {
        let block = BiasBlock::<f64>::default();
        assert_eq!(block.buffer(), 0.0);

        let block = BiasBlock::<Matrix<2, 2, f64>>::default();
        assert_eq!(block.buffer(), &Matrix::<2, 2, f64>::zeroed());
    }

    #[test]
    fn test_bias_scalar_to_pass() {
        let mut block = BiasBlock::<f64>::default();
        let parameters = Parameters::new(3.0);
        let context = StubContext::default();

        let output = block.process(&parameters, &context, -3.1);
        assert_relative_eq!(output, -0.1);
        assert_relative_eq!(block.buffer(), -0.1);
    }

    macro_rules! test_bias_block {
        // Convenience call to generate a call to the main macro for every type in the list
        ($type:ty, $($other_types:ty),*) => {
            test_bias_block!($type);
            test_bias_block!($($other_types),*);
        };
        ($type:ty) => {
            paste! {
                #[test]
                fn [<test_bias_scalar_ $type>]() {
                    let mut block = BiasBlock::<$type>::default();
                    let parameters = Parameters::new(3 as $type);
                    let context = StubContext::default();

                    let output = block.process(&parameters, &context, 2 as $type);
                    assert_eq!(output, 5 as $type);
                    assert_eq!(block.buffer(), output);
                }

                #[test]
                fn [<test_bias_matrix_ $type>]() {
                    let mut block = BiasBlock::<Matrix<2, 2, $type>>::default();
                    let context = StubContext::default();
                    let input = Matrix {
                        data: [[1 as $type, 2 as $type], [3 as $type, 4 as $type]],
                    };
                    let parameters = Parameters::new(2 as $type);
                    let output = block.process(&parameters, &context, &input);
                    let expected = [
                        [3 as $type, 4 as $type],
                        [5 as $type, 6 as $type],
                    ];
                    assert_eq!(output.data, expected);
                    assert_eq!(block.buffer().data, expected);
                }
            }
        };
    }

    test_bias_block!(f32, f64, i8, i16, i32, i64, u8, u16, u32, u64);

    #[test]
    #[should_panic]
    fn overflow_panics() {
        // Native integer addition overflow panics in debug builds (wraps in release).
        let mut block = BiasBlock::<u8>::default();
        let parameters = Parameters::new(1u8);
        let context = StubContext::default();

        let output = block.process(&parameters, &context, u8::MAX);
        assert_eq!(output, u8::MIN);
    }
}
