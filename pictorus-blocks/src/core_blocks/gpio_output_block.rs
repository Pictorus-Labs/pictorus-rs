use core::marker::PhantomData;

use block_data::BlockData as OldBlockData;
use pictorus_traits::{ByteSliceSignal, Context, Matrix, Pass, PassBy, ProcessBlock};

use crate::traits::Scalar;

/// Buffer data to be passed to a GPIO pin.
///
/// The block itself just handles converting the input to a boolean.
/// The hardware interaction happens downstream of this block.
pub struct GpioOutputBlock<T: ToBool> {
    pub data: OldBlockData,
    _unused: PhantomData<T>,
}

#[doc(hidden)]
pub struct Parameters {}
impl Default for Parameters {
    fn default() -> Self {
        Self::new()
    }
}

impl Parameters {
    pub fn new() -> Parameters {
        Parameters {}
    }
}

impl<T: ToBool> Default for GpioOutputBlock<T> {
    fn default() -> Self {
        GpioOutputBlock {
            _unused: PhantomData,
            data: OldBlockData::scalar_from_bool(false),
        }
    }
}

impl<T: ToBool> ProcessBlock for GpioOutputBlock<T> {
    type Parameters = Parameters;
    type Inputs = T;
    type Output = bool;

    fn process<'b>(
        &'b mut self,
        _parameters: &Self::Parameters,
        _context: &dyn Context,
        input: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        let res = T::to_bool(input);
        self.data.set_scalar_bool(res);
        res
    }
}

pub trait ToBool: Pass {
    fn to_bool(value: PassBy<Self>) -> bool;
}

impl<S: Scalar> ToBool for S {
    fn to_bool(value: PassBy<Self>) -> bool {
        value.is_truthy()
    }
}

impl ToBool for ByteSliceSignal {
    fn to_bool(value: PassBy<Self>) -> bool {
        !value.is_empty()
    }
}

impl<const NROWS: usize, const NCOLS: usize, S: Scalar> ToBool for Matrix<NROWS, NCOLS, S> {
    fn to_bool(value: PassBy<Self>) -> bool {
        for r in 0..NROWS {
            for c in 0..NCOLS {
                if value.data[c][r].is_truthy() {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::StubContext;
    use pictorus_traits::{ByteSliceSignal, Matrix};

    use super::*;

    #[test]
    fn test_gpio_output_block_scalar() {
        let mut block = GpioOutputBlock::<f64>::default();
        let context = StubContext::default();

        let output = block.process(&Parameters::new(), &context, 1.0);
        assert!(output);

        let output = block.process(&Parameters::new(), &context, 0.0);
        assert!(!output);
    }

    #[test]
    fn test_gpio_output_block_matrix() {
        let mut block = GpioOutputBlock::<Matrix<2, 2, f64>>::default();
        let context = StubContext::default();
        let input = Matrix {
            data: [[0.0, 0.0], [0.0, 1.0]],
        };

        let output = block.process(&Parameters::new(), &context, &input);
        assert!(output);

        let input = Matrix {
            data: [[0.0, 0.0], [0.0, 0.0]],
        };
        let output = block.process(&Parameters::new(), &context, &input);
        assert!(!output);
    }

    #[test]
    fn test_gpio_output_block_bytes() {
        let mut block = GpioOutputBlock::<ByteSliceSignal>::default();
        let context = StubContext::default();
        let params = Parameters::new();

        let output = block.process(&params, &context, b"hello world");
        assert!(output);

        let output = block.process(&params, &context, b"");
        assert!(!output);
    }
}
