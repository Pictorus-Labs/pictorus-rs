//! Public interfaces defining Pictorus block interactions
use crate::{Matrix, Scalar};
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
    fn run(&mut self, inputs: &[&dyn BlockDataRead], outputs: &mut [&mut dyn BlockDataWrite]);

    /// Optional cleanup of any resources used by this block
    ///
    /// This is useful if you would like to set some hardware state back to a default value before the app exits
    #[deprecated = "Users should use impl the core::ops::Drop trait instead"]
    fn cleanup(&mut self) {}
}

// Returns a 'static [f64] of length 1 backing the bool's f64 representation.
fn bool_as_matrix_data(b: bool) -> &'static [f64] {
    static LUT: [f64; 2] = [0.0, 1.0];
    core::slice::from_ref(&LUT[b as usize])
}

impl BlockDataRead for &bool {
    fn get_scalar(&self) -> f64 {
        if **self {
            1.0
        } else {
            0.0
        }
    }

    fn get_matrix(&self) -> (usize, usize, &[f64]) {
        (1, 1, bool_as_matrix_data(**self))
    }
}

impl BlockDataRead for bool {
    fn get_scalar(&self) -> f64 {
        if *self {
            1.0
        } else {
            0.0
        }
    }

    fn get_matrix(&self) -> (usize, usize, &[f64]) {
        (1, 1, bool_as_matrix_data(*self))
    }
}

macro_rules! scalar_block_data_read_impl {
        ($($t:ty),+) => {
            $(
                impl BlockDataRead for &$t {
                    fn get_scalar(&self) -> f64 {
                        (**self).into()
                    }
                    fn get_matrix(&self) -> (usize, usize, &[f64]) {
                        unimplemented!(
                            "get_matrix on scalar {} is not supported; only f64 and bool scalars convert to 1x1 matrices",
                            stringify!($t)
                        )
                    }
                }

                impl BlockDataRead for $t {
                    fn get_scalar(&self) -> f64 {
                        (*self).into()
                    }
                    fn get_matrix(&self) -> (usize, usize, &[f64]) {
                        unimplemented!(
                            "get_matrix on scalar {} is not supported; only f64 and bool scalars convert to 1x1 matrices",
                            stringify!($t)
                        )
                    }
                }
            )+
        };
    }

scalar_block_data_read_impl!(u8, i8, u16, i16, u32, i32, f32);

impl BlockDataRead for &f64 {
    fn get_scalar(&self) -> f64 {
        **self
    }

    fn get_matrix(&self) -> (usize, usize, &[f64]) {
        (1, 1, core::slice::from_ref(*self))
    }
}

impl BlockDataRead for f64 {
    fn get_scalar(&self) -> f64 {
        *self
    }

    fn get_matrix(&self) -> (usize, usize, &[f64]) {
        (1, 1, core::slice::from_ref(self))
    }
}

// We can't easily implement this for non f64 types because we need to have an array of f64
// with a long enough lifetime to be passed to the caller. We would need to use some sort of
// wrapper over the passed around data or over custom blocks to do this.
impl<const NROWS: usize, const NCOLS: usize> BlockDataRead for &Matrix<NROWS, NCOLS, f64> {
    fn get_scalar(&self) -> f64 {
        unimplemented!("Can not get scalar of matrix value")
    }

    fn get_matrix(&self) -> (usize, usize, &[f64]) {
        let data = self.data.as_flattened();
        (NROWS, NCOLS, data)
    }
}

impl<const NROWS: usize, const NCOLS: usize> BlockDataRead for Matrix<NROWS, NCOLS, f64> {
    fn get_scalar(&self) -> f64 {
        unimplemented!("Can not get scalar of matrix value")
    }

    fn get_matrix(&self) -> (usize, usize, &[f64]) {
        let data = self.data.as_flattened();
        (NROWS, NCOLS, data)
    }
}

impl BlockDataWrite for &mut bool {
    fn set_scalar_value(&mut self, value: f64) {
        **self = value != 0.0;
    }

    fn set_matrix_value(&mut self, _nrows: usize, _ncols: usize, _data: &[f64]) {
        unimplemented!("Can not set matrix of scalar bool value")
    }
}

impl BlockDataWrite for bool {
    fn set_scalar_value(&mut self, value: f64) {
        *self = value != 0.0;
    }

    fn set_matrix_value(&mut self, _nrows: usize, _ncols: usize, _data: &[f64]) {
        unimplemented!("Can not set matrix of scalar bool value")
    }
}

macro_rules! scalar_block_data_write_impl {
        ($($t:ty),+) => {
            $(
                impl BlockDataWrite for &mut $t {
                    fn set_scalar_value(&mut self, value: f64) {
                        // This is a lossy as cast which is currently our behavior elsewhere but still not ideal
                        **self = value as $t;
                    }
                    fn set_matrix_value(&mut self, _nrows: usize, _ncols: usize, _data: &[f64]) {
                        unimplemented!("Can not set matrix for scalar {}", stringify!($t))
                    }
                }

                impl BlockDataWrite for $t {
                    fn set_scalar_value(&mut self, value: f64) {
                        *self = value as $t;
                    }
                    fn set_matrix_value(&mut self, _nrows: usize, _ncols: usize, _data: &[f64]) {
                        unimplemented!("Can not set matrix for scalar {}", stringify!($t))
                    }
                }
            )+
        };
    }
scalar_block_data_write_impl!(u8, i8, u16, i16, u32, i32, f32, f64);

impl<const NROWS: usize, const NCOLS: usize, T: Scalar> BlockDataWrite
    for &mut Matrix<NROWS, NCOLS, T>
where
    for<'a> &'a mut T: BlockDataWrite,
{
    fn set_scalar_value(&mut self, _value: f64) {
        unimplemented!("Can not set scalar of matrix value")
    }

    fn set_matrix_value(&mut self, nrows: usize, ncols: usize, data: &[f64]) {
        assert_eq!(nrows, NROWS);
        assert_eq!(ncols, NCOLS);
        self.data
            .as_flattened_mut()
            .iter_mut()
            .zip(data.iter())
            .for_each(|(mut a, b)| a.set_scalar_value(*b));
    }
}

impl<const NROWS: usize, const NCOLS: usize, T: Scalar> BlockDataWrite for Matrix<NROWS, NCOLS, T>
where
    for<'a> &'a mut T: BlockDataWrite,
{
    fn set_scalar_value(&mut self, _value: f64) {
        unimplemented!("Can not set scalar of matrix value")
    }

    fn set_matrix_value(&mut self, nrows: usize, ncols: usize, data: &[f64]) {
        assert_eq!(nrows, NROWS);
        assert_eq!(ncols, NCOLS);
        self.data
            .as_flattened_mut()
            .iter_mut()
            .zip(data.iter())
            .for_each(|(mut a, b)| a.set_scalar_value(*b));
    }
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

    #[test]
    fn test_block_data_write_scalar_f64_through_dyn() {
        let mut x: f64 = 0.0;
        let writer: &mut dyn BlockDataWrite = &mut x;
        writer.set_scalar_value(42.5);
        assert_eq!(x, 42.5);
    }

    #[test]
    fn test_block_data_write_scalar_u8_through_dyn() {
        let mut x: u8 = 0;
        let writer: &mut dyn BlockDataWrite = &mut x;
        writer.set_scalar_value(7.0);
        assert_eq!(x, 7);
    }

    #[test]
    fn test_block_data_write_scalar_bool_through_dyn() {
        let mut x: bool = false;
        {
            let writer: &mut dyn BlockDataWrite = &mut x;
            writer.set_scalar_value(1.0);
        }
        assert!(x);
        {
            let writer: &mut dyn BlockDataWrite = &mut x;
            writer.set_scalar_value(0.0);
        }
        assert!(!x);
    }

    #[test]
    fn test_block_data_write_matrix_through_dyn() {
        let mut m: Matrix<2, 2, f64> = Matrix::zeroed();
        let writer: &mut dyn BlockDataWrite = &mut m;
        // Column-major: column 0 = [1.0, 3.0], column 1 = [2.0, 4.0]
        writer.set_matrix_value(2, 2, &[1.0, 3.0, 2.0, 4.0]);
        assert_eq!(m.data, [[1.0, 3.0], [2.0, 4.0]]);
    }

    #[test]
    fn test_block_data_read_scalar_f64_through_dyn() {
        let x: f64 = 3.5;
        let reader: &dyn BlockDataRead = &x;
        assert_eq!(reader.get_scalar(), 3.5);
    }

    #[test]
    fn test_block_data_read_scalar_bool_through_dyn() {
        let x = true;
        let reader: &dyn BlockDataRead = &x;
        assert_eq!(reader.get_scalar(), 1.0);
    }

    #[test]
    fn test_block_data_read_matrix_through_dyn() {
        let m: Matrix<2, 2, f64> = Matrix {
            data: [[1.0, 3.0], [2.0, 4.0]],
        };
        let reader: &dyn BlockDataRead = &m;
        let (nrows, ncols, data) = reader.get_matrix();
        assert_eq!(nrows, 2);
        assert_eq!(ncols, 2);
        assert_eq!(data, &[1.0, 3.0, 2.0, 4.0]);
    }

    #[test]
    fn test_block_data_read_scalar_f64_get_matrix_through_dyn() {
        let x: f64 = 3.5;
        let reader: &dyn BlockDataRead = &x;
        let (nrows, ncols, data) = reader.get_matrix();
        assert_eq!(nrows, 1);
        assert_eq!(ncols, 1);
        assert_eq!(data, &[3.5]);
    }

    #[test]
    fn test_block_data_read_scalar_f64_ref_get_matrix_through_dyn() {
        let value: f64 = 9.25;
        let x: &f64 = &value;
        let reader: &dyn BlockDataRead = &x;
        let (nrows, ncols, data) = reader.get_matrix();
        assert_eq!(nrows, 1);
        assert_eq!(ncols, 1);
        assert_eq!(data, &[9.25]);
    }

    #[test]
    fn test_block_data_read_scalar_bool_get_matrix_through_dyn() {
        let t = true;
        let reader: &dyn BlockDataRead = &t;
        let (nrows, ncols, data) = reader.get_matrix();
        assert_eq!(nrows, 1);
        assert_eq!(ncols, 1);
        assert_eq!(data, &[1.0]);

        let f = false;
        let reader: &dyn BlockDataRead = &f;
        let (nrows, ncols, data) = reader.get_matrix();
        assert_eq!(nrows, 1);
        assert_eq!(ncols, 1);
        assert_eq!(data, &[0.0]);
    }

    #[test]
    fn test_block_data_read_scalar_bool_ref_get_matrix_through_dyn() {
        let value = true;
        let x: &bool = &value;
        let reader: &dyn BlockDataRead = &x;
        let (nrows, ncols, data) = reader.get_matrix();
        assert_eq!(nrows, 1);
        assert_eq!(ncols, 1);
        assert_eq!(data, &[1.0]);
    }

    #[test]
    #[should_panic(expected = "get_matrix on scalar u8 is not supported")]
    fn test_block_data_read_scalar_u8_get_matrix_still_panics() {
        let x: u8 = 5;
        let reader: &dyn BlockDataRead = &x;
        let _ = reader.get_matrix();
    }
}
