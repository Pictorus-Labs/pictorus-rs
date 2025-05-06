use block_data::{BlockData as OldBlockData, FromPass};
use num_traits::{FromPrimitive, Zero};
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock, Scalar};

/// This block gets the index of the minimum or maximum value in the input.
/// For multidimensional inputs, the index returned is the linear index (i.e. the column-major order index).
///
/// ## Linear Index Example
/// The following 3x5 matrix has each element set to its "linear index"
/// ----------------------
/// | 0 | 3 | 6 | 9 | 12 |
/// | 1 | 4 | 7 | 10| 13 |
/// | 2 | 5 | 8 | 11| 14 |
/// ----------------------
pub struct ArgMinMaxBlock<T: Apply> {
    pub data: OldBlockData,
    buffer: Option<T::Output>,
}

impl<T: Apply> Default for ArgMinMaxBlock<T>
where
    T: Pass + Default,
    OldBlockData: FromPass<T::Output>,
{
    fn default() -> Self {
        Self {
            data: <OldBlockData as FromPass<T::Output>>::from_pass(<T::Output>::default().as_by()),
            buffer: None,
        }
    }
}

impl<T> ProcessBlock for ArgMinMaxBlock<T>
where
    T: Apply + Default,
    OldBlockData: FromPass<T::Output>,
{
    type Inputs = T;
    type Output = T::Output;
    type Parameters = Parameters;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        let output = T::apply(&mut self.buffer, inputs, parameters.method);
        self.data = OldBlockData::from_pass(output);
        output
    }
}

pub trait Apply: Pass {
    type Output: Scalar;

    fn apply<'s>(
        store: &mut Option<Self::Output>,
        input: PassBy<Self>,
        method: ArgMethod,
    ) -> PassBy<'s, Self::Output>;
}

trait ArgMinMaxScalar: Scalar + PartialOrd + Zero + FromPrimitive {}
impl ArgMinMaxScalar for f32 {}
impl ArgMinMaxScalar for f64 {}
impl ArgMinMaxScalar for i8 {}
impl ArgMinMaxScalar for i16 {}
impl ArgMinMaxScalar for i32 {}
impl ArgMinMaxScalar for u8 {}
impl ArgMinMaxScalar for u16 {}
impl ArgMinMaxScalar for u32 {}

impl<T: ArgMinMaxScalar> Apply for T {
    type Output = T;

    fn apply<'s>(
        store: &mut Option<Self::Output>,
        _input: PassBy<Self>,
        _method: ArgMethod,
    ) -> PassBy<'s, Self::Output> {
        // If a scalar is passed in then the only possible index is zero
        *store.insert(T::zero())
    }
}

impl<const NROWS: usize, const NCOLS: usize, T: ArgMinMaxScalar> Apply for Matrix<NROWS, NCOLS, T> {
    type Output = T;

    fn apply<'s>(
        store: &mut Option<Self::Output>,
        input: PassBy<Self>,
        method: ArgMethod,
    ) -> PassBy<'s, Self::Output> {
        let index = match method {
            ArgMethod::Min => {
                input
                    .data
                    .as_flattened()
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| a.partial_cmp(b).expect("Why did you give me a NaN!?"))
                    .expect("This iterator will never be empty")
                    .0
            }
            ArgMethod::Max => {
                input
                    .data
                    .as_flattened()
                    .iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("Why did you give me a NaN!?"))
                    .expect("This iterator will never be empty")
                    .0
            }
        };
        *store.insert(T::from_usize(index).expect("Couldn't convert usize to T"))
    }
}

#[derive(Debug, Clone, Copy, strum::EnumString)]
pub enum ArgMethod {
    Min,
    Max,
}

pub struct Parameters {
    pub method: ArgMethod,
}
impl Parameters {
    pub fn new(method: &str) -> Self {
        Self {
            method: method.parse().expect("Invalid min/max method"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;

    #[test]
    fn test_scalar_input() {
        let mut block = ArgMinMaxBlock::<f64>::default();
        let context = StubContext::default();
        let input = 1.0;
        let params = Parameters::new("Min");
        let output = block.process(&params, &context, input);
        assert_eq!(output, 0.0);
        assert_eq!(block.data.scalar(), 0.0);
    }

    #[test]
    fn test_matrix() {
        let context = StubContext::default();
        let mut block = ArgMinMaxBlock::<Matrix<2, 3, f64>>::default();
        // | 11  13  15 |
        // | 12   4  16 |
        // Min is 4 which is at linear index 3
        let input = Matrix {
            data: [[11.0, 12.0], [13.0, 4.0], [15.0, 16.0]],
        };
        let params = Parameters::new("Min");
        let output = block.process(&params, &context, &input);
        assert_eq!(output, 3.0);
        assert_eq!(block.data.scalar(), 3.0);

        // |  1  3  5 |
        // | 12  4  6 |
        // Max is 12 which is at linear index 1
        let input = Matrix {
            data: [[1.0, 12.0], [3.0, 4.0], [5.0, 6.0]],
        };
        let params = Parameters::new("Max");
        let output = block.process(&params, &context, &input);
        assert_eq!(output, 1.0);
        assert_eq!(block.data.scalar(), 1.0);
    }
}
