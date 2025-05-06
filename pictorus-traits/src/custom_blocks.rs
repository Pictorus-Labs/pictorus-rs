//! Public interfaces defining Pictorus block interactions
use core::ops::Index;

/// Param types passed into block constructors
#[derive(Debug, Clone)]
pub enum BlockParam<'a> {
    /// Scalar number value
    Number(f64),
    /// String value
    String(&'a str),
    /// Matrix value as a tuple of (nrows, ncols, `Vec<f64>`)
    Matrix(usize, usize, &'a [f64]),
}

impl BlockParam<'_> {
    pub fn as_number(&self) -> Option<f64> {
        match self {
            BlockParam::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            BlockParam::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_matrix(&self) -> Option<(usize, usize, &[f64])> {
        match self {
            BlockParam::Matrix(nrows, ncols, data) => Some((*nrows, *ncols, data)),
            _ => None,
        }
    }
}

/// Traits for setting and retrieving block data
pub trait BlockDataRead {
    // TODO: Not Sure I love this impl. I wonder if we want something that leverages enums instead, similar to how serde JSON impl works
    /// Retrieve a scalar value
    fn get_scalar(&self) -> f64;

    /// Retrieve a matrix value as a tuple of (nrows, ncols, &[f64])
    /// Data is output in column-major order
    /// For example, the matrix:
    ///     | 1.0 2.0 |
    ///     | 3.0 4.0 |
    ///
    /// will be returned as (2, 2, &[1.0, 3.0, 2.0, 4.0])
    fn get_matrix(&self) -> (usize, usize, &[f64]);
}

pub trait BlockDataWrite {
    /// Set a scalar value
    fn set_scalar_value(&mut self, value: f64);

    /// Set a matrix value
    /// Data is input in column-major order
    /// For example, set_matrix_value(2, 2, &[1.0, 3.0, 2.0, 4.0]) would set the matrixdata to:
    ///    | 1.0 2.0 |
    ///    | 3.0 4.0 |
    fn set_matrix_value(&mut self, nrows: usize, ncols: usize, data: &[f64]);
}

/// Trait for defining a block
// #[deprecated = "This trait is deprecated in favor of the <type>Block traits defined in lib.rs"]
pub trait BlockDef {
    /// Create a new block instance
    ///
    /// This receives the name and parameters of the associated block as specified in the
    /// Pictorus app UI.
    fn new(name: &'static str, params: &dyn Index<&str, Output = BlockParam>) -> Self;

    /// Run a single iteration of this block
    ///
    /// This receives a list of inputs corresponding to upstream blocks passing data into this block
    /// and a list of outputs corresponding to data that will be passed to downstream blocks.
    ///
    /// Each iteration of this block should modify the output data in place to reflect the current state
    fn run(&mut self, inputs: &[impl BlockDataRead], outputs: &mut [impl BlockDataWrite]);

    /// Optional cleanup of any resources used by this block
    ///
    /// This is useful if you would like to set some hardware state back to a default value before the app exits
    #[deprecated = "Users should use impl the core::ops::Drop trait instead"]
    fn cleanup(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_param_as_number() {
        let param = BlockParam::Number(42.0);
        assert_eq!(param.as_number(), Some(42.0));

        let param = BlockParam::String("not a number");
        assert_eq!(param.as_number(), None);

        let param = BlockParam::Matrix(2, 2, &[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(param.as_number(), None);
    }

    #[test]
    fn test_block_param_as_string() {
        let param = BlockParam::String("hello");
        assert_eq!(param.as_string(), Some("hello"));

        let param = BlockParam::Number(42.0);
        assert_eq!(param.as_string(), None);

        let param = BlockParam::Matrix(2, 2, &[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(param.as_string(), None);
    }

    #[test]
    fn test_block_param_as_matrix() {
        let param = BlockParam::Matrix(2, 2, &[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(
            param.as_matrix(),
            Some((2, 2, [1.0, 2.0, 3.0, 4.0].as_slice()))
        );

        let param = BlockParam::Number(42.0);
        assert_eq!(param.as_matrix(), None);

        let param = BlockParam::String("not a matrix");
        assert_eq!(param.as_matrix(), None);
    }
}
