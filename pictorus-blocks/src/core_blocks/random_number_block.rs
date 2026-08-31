use super::cast_block::CastElement;
use crate::traits::Scalar;
use num_traits::Float;
use pictorus_traits::{GeneratorBlock, PassBy};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use rand_distr::{Distribution, Normal, StandardNormal};

#[derive(Debug, Clone)]
/// Generates random numbers from a normal distribution with specified mean and standard deviation.
///
/// Sampling happens in the float type `F` and the sample is cast to the output type `T`
/// with `as` semantics. The mean is a value-domain parameter specified in `T`; std2
/// stays float so fractional spreads work for any output type.
pub struct RandomNumberBlock<T, F = T>
where
    T: Scalar + CastElement<F>,
    F: Scalar + Float + CastElement<T>,
    f64: From<F>,
    StandardNormal: Distribution<F>,
{
    phantom: core::marker::PhantomData<F>,
    rng: SmallRng,
    buffer: T,
}

impl<T, F> Default for RandomNumberBlock<T, F>
where
    T: Scalar + CastElement<F>,
    F: Scalar + Float + CastElement<T>,
    f64: From<F>,
    StandardNormal: Distribution<F>,
{
    fn default() -> Self {
        Self {
            phantom: core::marker::PhantomData,
            rng: SmallRng::seed_from_u64(0u64),
            buffer: T::default(),
        }
    }
}

impl<T, F> GeneratorBlock for RandomNumberBlock<T, F>
where
    T: Scalar + CastElement<F>,
    F: Scalar + Float + CastElement<T>,
    f64: From<F>,
    StandardNormal: Distribution<F>,
{
    type Output = T;
    type Parameters = Parameters<T, F>;

    fn generate(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
    ) -> pictorus_traits::PassBy<'_, Self::Output> {
        let mean: F = parameters.mean.cast_element();
        let val: F = self
            .rng
            //Will Fail if std2 is infinite: https://docs.rs/rand_distr/latest/src/rand_distr/normal.rs.html#156-161
            .sample(Normal::new(mean, parameters.std2).unwrap());
        self.buffer = val.cast_element();
        self.buffer
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        self.buffer
    }
}

pub struct Parameters<T: Scalar, F: Scalar = T> {
    pub mean: T,
    pub std2: F,
}

impl<T: Scalar, F: Scalar> Parameters<T, F> {
    pub fn new(mean: T, std2: F) -> Self {
        Self { mean, std2 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;

    #[test]
    fn test_random_number_default_buffer_no_panic() {
        let block = RandomNumberBlock::<f64>::default();
        assert_eq!(block.buffer(), 0.0);
    }

    #[test]
    fn test_random_number_block() {
        let stub_context = StubContext::default();
        // Just verify constructor and run method don't panic

        //f32
        let mut block = RandomNumberBlock::<f32>::default();
        let out = block.generate(&Parameters::new(1.0, 2.0), &stub_context);
        assert_eq!(block.buffer(), out);

        //f64
        let mut block = RandomNumberBlock::<f64>::default();
        block.generate(&Parameters::new(1.0, 2.0), &stub_context);
    }

    #[test]
    fn test_random_number_block_integer_output() {
        let stub_context = StubContext::default();

        // i32 output: sampled in f64 around an integer mean, cast at the output.
        // With std2 = 0 every sample is exactly the mean.
        let mut block = RandomNumberBlock::<i32, f64>::default();
        let out = block.generate(&Parameters::new(100i32, 0.0), &stub_context);
        assert_eq!(out, 100);
        assert_eq!(block.buffer(), out);
    }

    // This block's one arithmetic panic site is the Normal::new(..).unwrap() in
    // generate(): rand_distr rejects a non-finite std_dev (rand_distr-0.4.3
    // normal.rs:156-161). Everything else — the mean widening, the sampling, and the
    // output cast — is panic-free float math with saturating `as` casts.

    #[test]
    #[should_panic]
    fn test_infinite_std2_panics() {
        let stub_context = StubContext::default();
        let mut block = RandomNumberBlock::<f64>::default();
        block.generate(&Parameters::new(0.0, f64::INFINITY), &stub_context);
    }

    #[test]
    #[should_panic]
    fn test_nan_std2_panics() {
        let stub_context = StubContext::default();
        let mut block = RandomNumberBlock::<f64>::default();
        block.generate(&Parameters::new(0.0, f64::NAN), &stub_context);
    }

    #[test]
    fn test_negative_std2_does_not_panic() {
        // rand_distr accepts a negative std_dev (the distribution just mirrors), so a
        // negative std2 is not a panic site.
        let stub_context = StubContext::default();
        let mut block = RandomNumberBlock::<f64>::default();
        block.generate(&Parameters::new(0.0, -1.0), &stub_context);
    }

    #[test]
    fn test_random_number_block_mean_beyond_float_precision_rounds() {
        // A u64 mean above 2^53 is not exactly representable in the f64 the sample is
        // drawn in: 2^53 + 1 silently rounds to 2^53 (std2 = 0 isolates the round trip).
        let stub_context = StubContext::default();
        let mut block = RandomNumberBlock::<u64, f64>::default();
        let out = block.generate(&Parameters::new((1u64 << 53) + 1, 0.0), &stub_context);
        assert_eq!(out, 1u64 << 53);
    }

    #[test]
    fn test_random_number_block_integer_output_saturates() {
        // A sample far outside the output type's range saturates at the type's limits
        // rather than wrapping or panicking — the distribution's tails are clipped.
        let stub_context = StubContext::default();
        let mut block = RandomNumberBlock::<i8, f64>::default();
        let out = block.generate(&Parameters::new(0i8, 1e30), &stub_context);
        assert!(out == i8::MIN || out == i8::MAX);
    }
}
