use num_traits::Float;
use pictorus_block_data::BlockData;
use pictorus_traits::{GeneratorBlock, Scalar};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use rand_distr::{Distribution, Normal, StandardNormal};

#[derive(Debug, Clone)]
/// Generates random numbers from a normal distribution with specified mean and standard deviation.
pub struct RandomNumberBlock<T>
where
    T: Scalar + Float,
    f64: From<T>,
    StandardNormal: Distribution<T>,
{
    phantom: core::marker::PhantomData<T>,
    rng: SmallRng,
    pub data: BlockData,
}

impl<T> Default for RandomNumberBlock<T>
where
    T: Scalar + Float,
    f64: From<T>,
    StandardNormal: Distribution<T>,
{
    fn default() -> Self {
        Self {
            phantom: core::marker::PhantomData,
            rng: SmallRng::seed_from_u64(0u64),
            data: BlockData::from_scalar(f64::from(T::default())),
        }
    }
}

impl<T> GeneratorBlock for RandomNumberBlock<T>
where
    T: Scalar + Float,
    f64: From<T>,
    StandardNormal: Distribution<T>,
{
    type Output = T;
    type Parameters = Parameters<T>;

    fn generate(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
    ) -> pictorus_traits::PassBy<Self::Output> {
        let val = self
            .rng
            //Will Fail if std2 is infinite: https://docs.rs/rand_distr/latest/src/rand_distr/normal.rs.html#156-161
            .sample(Normal::new(parameters.mean, parameters.std2).unwrap());
        self.data = BlockData::from_scalar(f64::from(val));
        val
    }
}

pub struct Parameters<T: Scalar> {
    pub mean: T,
    pub std2: T,
}

impl<T: Scalar> Parameters<T> {
    pub fn new(mean: T, std2: T) -> Self {
        Self { mean, std2 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;

    #[test]
    fn test_random_number_block() {
        let stub_context = StubContext::default();
        // Just verify constructor and run method don't panic

        //f32
        let mut block = RandomNumberBlock::<f32>::default();
        block.generate(&Parameters::new(1.0, 2.0), &stub_context);

        //f64
        let mut block = RandomNumberBlock::<f64>::default();
        block.generate(&Parameters::new(1.0, 2.0), &stub_context);
    }
}
