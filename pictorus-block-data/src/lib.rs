//! This crate contains deprecated code for passing data between blocks.
//! We are in the process of phasing this out in favor of an approach that
//! allows us to pass data around without the use of dynamic heap allocations.
#![no_std]

extern crate alloc;

mod block_ops;

use alloc::borrow::ToOwned;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cmp::Ordering;
use num_traits::Float;

use miniserde::json::{self, Array, Number, Value};
use nalgebra::iter::MatrixIter;
use nalgebra::{DMatrix, SVD};

use pictorus_traits::custom_blocks::{BlockDataRead, BlockDataWrite};
use pictorus_traits::{ByteSliceSignal, PassBy};

#[derive(Clone, Debug, PartialEq, Copy, strum::EnumString)]
pub enum BlockDataType {
    BytesArray,
    Scalar,
    Vector,
    Matrix,
}

pub enum BlockTypeRelationship {
    SameSizes,
    FirstIsScalar,
    SecondIsScalar,
    None,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlockData {
    _data: DMatrix<f64>,
    _type: BlockDataType,
}

impl BlockDataRead for BlockData {
    fn get_scalar(&self) -> f64 {
        self.scalar()
    }

    fn get_matrix(&self) -> (usize, usize, &[f64]) {
        (self.nrows(), self.ncols(), self._data.as_slice())
    }
}

impl BlockDataRead for &BlockData {
    fn get_scalar(&self) -> f64 {
        self.scalar()
    }

    fn get_matrix(&self) -> (usize, usize, &[f64]) {
        (self.nrows(), self.ncols(), self._data.as_slice())
    }
}

impl BlockDataWrite for &mut BlockData {
    fn set_scalar_value(&mut self, scalar: f64) {
        self.set_scalar(scalar);
    }

    fn set_matrix_value(&mut self, nrows: usize, ncols: usize, data: &[f64]) {
        self._data = DMatrix::from_column_slice(nrows, ncols, data);
        self._type = determine_data_type(nrows, ncols);
    }
}

pub fn determine_data_type(nrows: usize, ncols: usize) -> BlockDataType {
    if nrows == 1 && ncols == 1 {
        BlockDataType::Scalar
    } else if nrows == 1 {
        BlockDataType::Vector
    } else {
        BlockDataType::Matrix
    }
}

fn create_byte_row(byte_data: &[u8]) -> DMatrix<f64> {
    let data: Vec<f64> = byte_data.iter().map(|&v| v as f64).collect();
    DMatrix::from_row_slice(1, data.len(), &data)
}

impl BlockData {
    pub fn new(rows: usize, cols: usize, data: &[f64]) -> Self {
        Self::from_row_slice(rows, cols, data)
    }

    pub fn from_data(data: DMatrix<f64>, dtype: BlockDataType) -> Self {
        Self {
            _data: data,
            _type: dtype,
        }
    }
    pub fn from_element(nrows: usize, ncols: usize, scalar: f64) -> Self {
        Self::from_data(
            DMatrix::from_element(nrows, ncols, scalar),
            determine_data_type(nrows, ncols),
        )
    }
    pub fn from_scalar(scalar: f64) -> Self {
        Self::new(1, 1, &[scalar])
    }
    pub fn from_row_slice(nrows: usize, ncols: usize, data: &[f64]) -> Self {
        Self::from_data(
            DMatrix::from_row_slice(nrows, ncols, data),
            determine_data_type(nrows, ncols),
        )
    }

    pub fn from_bytes(byte_data: &[u8]) -> Self {
        Self::from_data(create_byte_row(byte_data), BlockDataType::BytesArray)
    }

    pub fn scalar_sizeof(scalar: f64, block: &BlockData) -> Self {
        Self::from_element(block.nrows(), block.ncols(), scalar)
    }

    pub fn from_vector(data: &[f64]) -> Self {
        Self::new(1, data.len(), data)
    }

    pub fn from_matrix(matrix_slice: &[&[f64]]) -> Self {
        let rows = matrix_slice.len();
        let cols = if rows > 0 { matrix_slice[0].len() } else { 0 };

        Self::from_data(
            DMatrix::from_row_iterator(rows, cols, matrix_slice.concat()),
            BlockDataType::Matrix,
        )
    }

    pub fn zeros_sizeof(block: &BlockData) -> Self {
        Self::from_element(block.nrows(), block.ncols(), 0.0)
    }
    pub fn ones_sizeof(block: &BlockData) -> Self {
        Self::from_element(block.nrows(), block.ncols(), 1.0)
    }

    pub fn scalar_from_bool(value: bool) -> Self {
        let value = if value { 1.0 } else { 0.0 };
        Self::from_scalar(value)
    }

    pub fn fix_non_finite(&mut self) {
        self._data = self._data.map(|x| {
            if x.is_nan() || x == f64::INFINITY || x == f64::NEG_INFINITY {
                0.0
            } else {
                x
            }
        });
    }

    pub fn slice(&mut self, r0: usize, c0: usize, rows: usize, cols: usize) -> Self {
        let slice = self._data.view((r0, c0), (rows, cols)).to_owned().into();
        Self::from_data(slice, determine_data_type(rows, cols))
    }

    pub fn as_col_slice(&self) -> &[f64] {
        self._data.as_slice()
    }

    pub fn set(&mut self, index: usize, value: f64) {
        self._data[index] = value;
    }
    pub fn at(&self, index: usize) -> f64 {
        self._data[index]
    }
    pub fn ref_at(&self, index: usize) -> &f64 {
        &self._data[index]
    }
    pub fn ref_at_mut(&mut self, index: usize) -> &mut f64 {
        &mut self._data[index]
    }
    pub fn at_rc(&self, r: usize, c: usize) -> f64 {
        self._data[(r, c)]
    }
    pub fn get_type(&self) -> BlockDataType {
        self._type
    }
    pub fn set_type(&mut self, new_type: BlockDataType) {
        self._type = new_type;
    }
    pub fn scalar(&self) -> f64 {
        // Scalars just return the (0,0)th element of the matrix
        match self.get_type() {
            BlockDataType::Scalar => self.at(0),
            other => panic!("Cannot treat {:?} as scalar!", other),
        }
    }
    pub fn get_data(&self) -> &DMatrix<f64> {
        &self._data
    }
    pub fn set_data(&mut self, data: DMatrix<f64>) {
        self._data = data;
    }
    pub fn maybe_reset(&mut self, reset_signal: &BlockData) {
        // Resets elements of data where reset_signal is truthy
        if reset_signal.any() {
            self._data = self.component_mul(&reset_signal.logical_not())._data;
        }
    }
    pub fn compare(&self, other: &BlockData) -> BlockTypeRelationship {
        if self.same_size(other) {
            return BlockTypeRelationship::SameSizes;
        } else if self.get_type() == BlockDataType::Scalar {
            return BlockTypeRelationship::FirstIsScalar;
        } else if other.get_type() == BlockDataType::Scalar {
            return BlockTypeRelationship::SecondIsScalar;
        }
        BlockTypeRelationship::None
    }
    pub fn vector(&self) -> DMatrix<f64> {
        // Scalars just return the (0,0)th element of the matrix
        match self.get_type() {
            BlockDataType::Vector => self.get_data().clone(),
            _ => panic!("Cannot treat non-scalar BlockData as scalar!"),
        }
    }

    pub fn boolean(&self) -> Self {
        BlockData::component_neq(self, &BlockData::zeros_sizeof(self))
    }

    pub fn norm(&self) -> f64 {
        self._data.norm()
    }

    pub fn vector_magnitude(&self) -> Self {
        /*
        Returns a BlockData representing the vector magnitude of each column of Self
        */

        let mut magnitudes = BlockData::from_element(1, self.ncols(), 0.0);
        for i in 0..self.ncols() {
            let column = self._data.column(i);
            let magnitude = column.norm();
            magnitudes[i] = magnitude;
        }

        magnitudes
    }

    pub fn vector_magnitude_rows(&self) -> Self {
        /*
        Returns a BlockData representing the vector magnitude of each row of Self
        */
        self.transpose().vector_magnitude()
    }

    pub fn any(&self) -> bool {
        self.boolean().sum() > 0.0
    }

    pub fn all(&self) -> bool {
        self.boolean().sum() == (self.nrows() * self.ncols()) as f64
    }

    pub fn set_scalar(&mut self, new_scalar_value: f64) {
        match self.get_type() {
            BlockDataType::Scalar => self._data[(0, 0)] = new_scalar_value,
            _ => panic!("Cannot treat non-scalar BlockData as scalar!"),
        }
    }
    pub fn set_bytes(&mut self, byte_data: &[u8]) {
        match self.get_type() {
            BlockDataType::BytesArray => {
                self._data = create_byte_row(byte_data);
            }
            _ => panic!("Cannot treat bytes as numeric data"),
        }
    }
    pub fn set_scalar_bool(&mut self, value: bool) {
        let value = if value { 1.0 } else { 0.0 };
        self.set_scalar(value);
    }
    pub fn nrows(&self) -> usize {
        self._data.nrows()
    }
    pub fn ncols(&self) -> usize {
        self._data.ncols()
    }
    pub fn size(&self) -> (usize, usize) {
        (self.nrows(), self.ncols())
    }
    pub fn n_elements(&self) -> usize {
        self.nrows() * self.ncols()
    }
    pub fn same_size(&self, other: &BlockData) -> bool {
        (self.nrows() == other.nrows()) && (self.ncols() == other.ncols())
    }
    pub fn inner_dims_same(&self, other: &BlockData) -> bool {
        self.ncols() == other.nrows()
    }
    pub fn component_mul(&self, other: &BlockData) -> Self {
        match self.compare(other) {
            BlockTypeRelationship::SameSizes => {
                BlockData::from_data(self._data.component_mul(&other._data), self._type)
            }
            BlockTypeRelationship::FirstIsScalar => {
                BlockData::from_data(self.scalar() * other.get_data(), other.get_type())
            }
            BlockTypeRelationship::SecondIsScalar => {
                BlockData::from_data(&self._data * other.scalar(), self._type)
            }
            _ => panic!(
                "Cannot perform component-wise multiplication if (a) sizes are not equal or (b) at least one is not scalar."
            ),
        }
    }
    pub fn component_set(&mut self, condition: &BlockData, assign: &BlockData) {
        // Conditionally set each element: If "condition" is not falsy, then set to the
        // value specified in "assign"
        match condition.compare(assign) {
            BlockTypeRelationship::SameSizes => {
                for (idx, condition_val) in condition._data.iter().enumerate() {
                    if *condition_val != 0.0 {
                        self._data[idx] = assign.at(idx);
                    }
                }
            }
            // TODO: Should eventually allow scalar to matrix assignment
            _ => panic!("Cannot perform component-wise assignment if sizes are not equal."),
        }
    }
    pub fn component_or(first: &BlockData, other: &BlockData) -> Self {
        match first.compare(other) {
            BlockTypeRelationship::SameSizes => {
                let mut result = BlockData::zeros_sizeof(first);
                for (idx, value) in first._data.iter().enumerate() {
                    let val = if *value != 0.0 || other.at(idx) != 0.0 {
                        1.0
                    } else {
                        0.0
                    };
                    result.set(idx, val);
                }
                result
            }
            // TODO: Should eventually allow scalar to matrix comparisons
            _ => panic!("Cannot perform component-wise comparisons if sizes are not equal."),
        }
    }
    pub fn component_and(first: &BlockData, other: &BlockData) -> Self {
        match first.compare(other) {
            BlockTypeRelationship::SameSizes => {
                let mut result = BlockData::zeros_sizeof(first);
                for (idx, value) in first._data.iter().enumerate() {
                    let val = if *value != 0.0 && other.at(idx) != 0.0 {
                        1.0
                    } else {
                        0.0
                    };
                    result.set(idx, val);
                }
                result
            }
            // TODO: Should eventually allow scalar to matrix comparisons
            _ => panic!("Cannot perform component-wise comparisons if sizes are not equal."),
        }
    }

    pub fn component_bitand(&self, rhs: &BlockData) -> Self {
        self.component_map(rhs, |i1, i2| (i1 as i64 & i2 as i64) as f64)
    }

    pub fn component_bitor(&self, rhs: &BlockData) -> Self {
        self.component_map(rhs, |i1, i2| (i1 as i64 | i2 as i64) as f64)
    }

    pub fn component_bitxor(&self, rhs: &BlockData) -> Self {
        self.component_map(rhs, |i1, i2| (i1 as i64 ^ i2 as i64) as f64)
    }

    pub fn component_lshift(&self, bits: i32) -> Self {
        BlockData::from_data(self._data.map(|d| ((d as i64) << bits) as f64), self._type)
    }

    pub fn component_rshift(&self, bits: i32) -> Self {
        BlockData::from_data(self._data.map(|d| ((d as i64) >> bits) as f64), self._type)
    }

    pub fn component_bitnot(&self) -> Self {
        BlockData::from_data(self._data.map(|d| !(d as i64) as f64), self._type)
    }

    fn component_map<F>(&self, rhs: &BlockData, f: F) -> BlockData
    where
        F: Fn(f64, f64) -> f64,
    {
        match self.compare(rhs) {
            BlockTypeRelationship::SameSizes => {
                BlockData::from_data(self._data.zip_map(rhs.get_data(), f), self._type)
            }
            _ => panic!("Cannot perform component-wise comparisons if sizes are not equal."),
        }
    }
    pub fn component_eq(first: &BlockData, other: &BlockData) -> Self {
        match first.compare(other) {
            BlockTypeRelationship::SameSizes => BlockData::from_data(
                DMatrix::from_fn(first.nrows(), first.ncols(), |i, j| {
                    (Float::abs(first.at_rc(i, j) - other.at_rc(i, j)) < f64::EPSILON) as u8 as f64
                }),
                first.get_type(),
            ),
            // TODO: Should eventually allow scalar to matrix comparisons
            _ => panic!("Cannot perform component-wise comparisons if sizes are not equal."),
        }
    }
    pub fn component_neq(first: &BlockData, other: &BlockData) -> Self {
        match first.compare(other) {
            BlockTypeRelationship::SameSizes => BlockData::from_data(
                DMatrix::from_fn(first.nrows(), first.ncols(), |i, j| {
                    (Float::abs(first.at_rc(i, j) - other.at_rc(i, j)) >= f64::EPSILON) as u8 as f64
                }),
                first.get_type(),
            ),
            // TODO: Should eventually allow scalar to matrix comparisons
            _ => panic!("Cannot perform component-wise comparisons if sizes are not equal."),
        }
    }

    pub fn component_gt(first: &BlockData, other: &BlockData) -> Self {
        match first.compare(other) {
            BlockTypeRelationship::SameSizes => {
                let mut result = BlockData::zeros_sizeof(first);
                for (idx, value) in first._data.iter().enumerate() {
                    let val = if *value > other.at(idx) { 1.0 } else { 0.0 };
                    result.set(idx, val);
                }
                result
            }
            // TODO: Should eventually allow scalar to matrix comparisons
            _ => panic!("Cannot perform component-wise comparisons if sizes are not equal."),
        }
    }

    pub fn component_gte(first: &BlockData, other: &BlockData) -> Self {
        match first.compare(other) {
            BlockTypeRelationship::SameSizes => {
                let mut result = BlockData::zeros_sizeof(first);
                for (idx, value) in first._data.iter().enumerate() {
                    let val = if *value >= other.at(idx) { 1.0 } else { 0.0 };
                    result.set(idx, val);
                }
                result
            }
            // TODO: Should eventually allow scalar to matrix comparisons
            _ => panic!("Cannot perform component-wise comparisons if sizes are not equal."),
        }
    }

    pub fn component_lt(first: &BlockData, other: &BlockData) -> Self {
        match first.compare(other) {
            BlockTypeRelationship::SameSizes => {
                let mut result = BlockData::zeros_sizeof(first);
                for (idx, value) in first._data.iter().enumerate() {
                    let val = if *value < other.at(idx) { 1.0 } else { 0.0 };
                    result.set(idx, val);
                }
                result
            }
            // TODO: Should eventually allow scalar to matrix comparisons
            _ => panic!("Cannot perform component-wise comparisons if sizes are not equal."),
        }
    }

    pub fn component_lte(first: &BlockData, other: &BlockData) -> Self {
        match first.compare(other) {
            BlockTypeRelationship::SameSizes => {
                let mut result = BlockData::zeros_sizeof(first);
                for (idx, value) in first._data.iter().enumerate() {
                    let val = if *value <= other.at(idx) { 1.0 } else { 0.0 };
                    result.set(idx, val);
                }
                result
            }
            // TODO: Should eventually allow scalar to matrix comparisons
            _ => panic!("Cannot perform component-wise comparisons if sizes are not equal."),
        }
    }

    pub fn powf(&self, coefficient: &BlockData) -> Self {
        // TODO: Nalgebra currently only supports integer exponentiation.
        // Rolling our own to handle floating point.
        let mut result = BlockData::zeros_sizeof(self);
        for (i, value) in self._data.iter().enumerate() {
            result.set(i, Float::powf(*value, coefficient.at(i)));
        }
        result
    }
    pub fn sign(&self) -> Self {
        // Return f64 {1.0, 0.0, -1.0} to represent sign of each element in BlockData
        // as per numpy convention
        let mut sign_vals = self.gtz();
        sign_vals.component_set(&self.ltz(), &BlockData::scalar_sizeof(-1.0, self));
        sign_vals
    }
    pub fn abs(&self) -> Self {
        BlockData::from_data(self._data.abs(), self._type)
    }
    pub fn transpose(&self) -> Self {
        BlockData::from_data(self._data.transpose(), self._type)
    }
    pub fn determinant(&self) -> Self {
        let det = self._data.determinant();
        BlockData::from_scalar(det)
    }
    pub fn cross(&self, other: &BlockData) -> Self {
        BlockData::from_data(self._data.cross(&other._data), self._type)
    }
    pub fn dot(&self, other: &BlockData) -> Self {
        let dot: f64 = self._data.dot(&other._data);
        BlockData::from_scalar(dot)
    }
    pub fn inverse(&self) -> Option<Self> {
        let inverse = self._data.clone().try_inverse()?;
        Some(BlockData::from_data(inverse, self._type))
    }
    pub fn pseudo_inverse(&self, epsilon: f64) -> Option<Self> {
        let svd = SVD::new(self._data.clone(), true, true);
        let s = svd.singular_values;
        let u = svd.u?;
        let v_t = svd.v_t?;
        let s_pinv = DMatrix::from_diagonal(&s.map(|x| {
            if Float::abs(x) > epsilon {
                1.0 / x
            } else {
                0.0
            }
        }));
        let pinv = u * s_pinv * v_t;
        Some(BlockData::from_data(pinv.transpose(), self._type))
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        match self.get_type() {
            BlockDataType::BytesArray => self._data.iter().map(|&v| v as u8).collect(),
            _ => self.stringify().as_bytes().to_vec(),
        }
    }

    /// Returns a Vector of bytes representing the raw data
    pub fn to_raw_bytes(&self) -> Vec<u8> {
        self._data.iter().map(|&v| v as u8).collect()
    }

    /// Returns the serde:json::Value equivalent of this data
    pub fn to_json(&self) -> Value {
        match self.get_type() {
            BlockDataType::Scalar => Value::Number(Number::F64(self.scalar())),
            BlockDataType::BytesArray => {
                let mut arr = Array::new();
                self.to_bytes()
                    .iter()
                    .for_each(|v| arr.push(Value::Number(Number::U64(*v as u64))));
                Value::Array(arr)
            }
            _ => {
                let mut arr = Array::new();
                self._data.row_iter().for_each(|r| {
                    let mut inner_array = Array::new();
                    r.iter()
                        .for_each(|v| inner_array.push(Value::Number(Number::F64(*v))));
                    arr.push(Value::Array(inner_array));
                });
                Value::Array(arr)
            }
        }
    }

    /// Returns the data represented as a valid JSON string
    pub fn stringify(&self) -> String {
        json::to_string(&self.to_json())
    }

    /// Return the raw data represented as a string
    ///
    /// This will attempt to convert internal vector data to utf-8. This behaves the same as stringify
    /// for all data types other than byte arrays, which will attempt to convert to a string.
    pub fn raw_string(&self) -> String {
        String::from_utf8(self.to_bytes()).unwrap_or("".to_string())
    }

    pub fn iter(
        &self,
    ) -> MatrixIter<
        '_,
        f64,
        nalgebra::Dyn,
        nalgebra::Dyn,
        nalgebra::VecStorage<f64, nalgebra::Dyn, nalgebra::Dyn>,
    > {
        self._data.iter()
    }

    pub fn map<F: FnMut(f64) -> f64>(&self, f: F) -> Self {
        BlockData::from_data(self._data.map(f), self._type)
    }

    pub fn sum(&self) -> f64 {
        self._data.sum()
    }

    pub fn mean(&self) -> f64 {
        self.sum() / self.n_elements() as f64
    }

    pub fn max(&self) -> f64 {
        self._data.iter().cloned().fold(f64::MIN, f64::max)
    }

    pub fn min(&self) -> f64 {
        self._data.iter().cloned().fold(f64::MAX, f64::min)
    }

    pub fn argmax(&self) -> f64 {
        self._data
            .iter()
            .enumerate()
            .fold((0, f64::MIN), |(max_idx, max_val), (idx, &val)| {
                if val > max_val {
                    (idx, val)
                } else {
                    (max_idx, max_val)
                }
            })
            .0 as f64
    }

    pub fn argmin(&self) -> f64 {
        self._data
            .iter()
            .enumerate()
            .fold((0, f64::MAX), |(min_idx, min_val), (idx, &val)| {
                if val < min_val {
                    (idx, val)
                } else {
                    (min_idx, min_val)
                }
            })
            .0 as f64
    }

    pub fn sorted(&self, ascending: bool) -> Self {
        let mut sorted_values: Vec<f64> = self._data.iter().copied().collect();

        // sort_unstable_by is more memory efficient, and is only "unstable" in that
        // it doesn't preserve order of equal values. As this isn't really an issue for
        // us, we'll take the memory win.
        sorted_values.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Less));

        if !ascending {
            sorted_values.reverse();
        }

        BlockData::from_vector(sorted_values.as_slice())
    }

    pub fn median(&self) -> f64 {
        let sorted_values = self.sorted(true);
        let len = sorted_values.len();
        if len % 2 == 0 {
            (sorted_values.at(len / 2 - 1) + sorted_values.at(len / 2)) / 2.0
        } else {
            sorted_values.at(len / 2)
        }
    }

    pub fn sup(&self, other: &BlockData) -> Self {
        BlockData::from_data(self._data.sup(&other._data), self._type)
    }

    pub fn inf(&self, other: &BlockData) -> Self {
        BlockData::from_data(self._data.inf(&other._data), self._type)
    }

    pub fn len(&self) -> usize {
        self._data.len()
    }

    pub fn is_empty(&self) -> bool {
        self._data.is_empty()
    }

    pub fn or(&self, other: &BlockData) -> Self {
        BlockData::component_or(self, other)
    }

    pub fn and(&self, other: &BlockData) -> Self {
        BlockData::component_and(self, other)
    }

    pub fn eq(&self, other: &BlockData) -> Self {
        BlockData::component_eq(self, other)
    }

    pub fn neq(&self, other: &BlockData) -> Self {
        BlockData::component_neq(self, other)
    }

    pub fn gt(&self, other: &BlockData) -> Self {
        BlockData::component_gt(self, other)
    }

    pub fn gte(&self, other: &BlockData) -> Self {
        BlockData::component_gte(self, other)
    }

    pub fn lt(&self, other: &BlockData) -> Self {
        BlockData::component_lt(self, other)
    }

    pub fn lte(&self, other: &BlockData) -> Self {
        BlockData::component_lte(self, other)
    }

    pub fn logical_not(&self) -> Self {
        // Not is just checking every value for equality with zero (false)
        self.eqz()
    }

    pub fn ltz(&self) -> Self {
        // Less than zero
        BlockData::from_data(self._data.map(|d| ((d as i8) < 0) as u8 as f64), self._type)
    }

    pub fn ltez(&self) -> Self {
        // Less than or equal to zero
        BlockData::from_data(
            self._data.map(|d| ((d as i8) <= 0) as u8 as f64),
            self._type,
        )
    }

    pub fn gtz(&self) -> Self {
        // Greater than zero
        BlockData::from_data(self._data.map(|d| ((d as i8) > 0) as u8 as f64), self._type)
    }

    pub fn gtez(&self) -> Self {
        // Greater than or equal to zero
        BlockData::from_data(
            self._data.map(|d| ((d as i8) >= 0) as u8 as f64),
            self._type,
        )
    }

    pub fn eqz(&self) -> Self {
        // Equal to zero
        BlockData::from_data(
            self._data.map(|d| ((d as i8) == 0) as u8 as f64),
            self._type,
        )
    }
}

pub trait FromPass<T: pictorus_traits::Pass> {
    fn from_pass(pass: PassBy<T>) -> Self;
}

impl FromPass<f64> for BlockData {
    fn from_pass(pass: f64) -> Self {
        BlockData::from_scalar(pass)
    }
}

impl FromPass<f32> for BlockData {
    fn from_pass(pass: f32) -> Self {
        BlockData::from_scalar(pass.into())
    }
}

impl FromPass<u8> for BlockData {
    fn from_pass(pass: u8) -> Self {
        BlockData::from_scalar(pass.into())
    }
}

impl FromPass<i8> for BlockData {
    fn from_pass(pass: i8) -> Self {
        BlockData::from_scalar(pass.into())
    }
}

impl FromPass<u16> for BlockData {
    fn from_pass(pass: u16) -> Self {
        BlockData::from_scalar(pass.into())
    }
}

impl FromPass<i16> for BlockData {
    fn from_pass(pass: i16) -> Self {
        BlockData::from_scalar(pass.into())
    }
}

impl FromPass<u32> for BlockData {
    fn from_pass(pass: u32) -> Self {
        BlockData::from_scalar(pass.into())
    }
}

impl FromPass<i32> for BlockData {
    fn from_pass(pass: i32) -> Self {
        BlockData::from_scalar(pass.into())
    }
}

impl FromPass<bool> for BlockData {
    fn from_pass(pass: PassBy<bool>) -> Self {
        let scalar = if pass { 1. } else { 0. };
        BlockData::from_scalar(scalar)
    }
}

impl FromPass<ByteSliceSignal> for BlockData {
    fn from_pass(pass: PassBy<ByteSliceSignal>) -> Self {
        BlockData::from_bytes(pass)
    }
}

impl<const NROWS: usize, const NCOLS: usize, T: pictorus_traits::Scalar>
    FromPass<pictorus_traits::Matrix<NROWS, NCOLS, T>> for BlockData
{
    fn from_pass(pass: PassBy<pictorus_traits::Matrix<NROWS, NCOLS, T>>) -> Self {
        let mut data = DMatrix::<f64>::zeros(NROWS, NCOLS);
        for i in 0..NROWS {
            for j in 0..NCOLS {
                // Note the i,j <-> j,i here is not a bug but is due to Matrix storing data as `[[T; NROWS]; NCOLS]` so the first  `[]` indexes into the Columns
                data[(i, j)] = pass.data[j][i].into();
            }
        }
        BlockData::from_data(data, BlockDataType::Matrix)
    }
}

pub trait ToPass<T: pictorus_traits::Pass> {
    fn to_pass(&self) -> T;
}

impl ToPass<f64> for BlockData {
    fn to_pass(&self) -> f64 {
        self.scalar()
    }
}

impl ToPass<bool> for BlockData {
    fn to_pass(&self) -> bool {
        self.scalar() != 0.0
    }
}

impl<const N: usize> ToPass<[u8; N]> for BlockData {
    fn to_pass(&self) -> [u8; N] {
        let mut data = [0; N];
        for (i, v) in self._data.iter().enumerate() {
            data[i] = *v as u8;
        }
        data
    }
}

impl<const N: usize> ToPass<[f64; N]> for BlockData {
    fn to_pass(&self) -> [f64; N] {
        let mut data = [0.0; N];
        for (i, v) in self._data.iter().enumerate() {
            data[i] = *v;
        }
        data
    }
}

impl<const NROWS: usize, const NCOLS: usize> ToPass<pictorus_traits::Matrix<NROWS, NCOLS, f64>>
    for BlockData
{
    fn to_pass(&self) -> pictorus_traits::Matrix<NROWS, NCOLS, f64> {
        let mut data = pictorus_traits::Matrix::<NROWS, NCOLS, f64>::zeroed();
        for i in 0..NROWS {
            for j in 0..NCOLS {
                // See the comment on `FromPass` for explanation of why indexes are switched
                data.data[j][i] = self.at_rc(i, j);
            }
        }
        data
    }
}

impl<const NROWS: usize, const NCOLS: usize> ToPass<pictorus_traits::Matrix<NROWS, NCOLS, bool>>
    for BlockData
{
    fn to_pass(&self) -> pictorus_traits::Matrix<NROWS, NCOLS, bool> {
        let mut data = pictorus_traits::Matrix::<NROWS, NCOLS, bool>::zeroed();
        for i in 0..NROWS {
            for j in 0..NCOLS {
                // See the comment on `FromPass` for explanation of why indexes are switched
                data.data[j][i] = self.at_rc(i, j) != 0.0;
            }
        }
        data
    }
}

pub fn all_blocks_same_size(blocks: Vec<&BlockData>) -> bool {
    for block in blocks.iter() {
        if !block.same_size(blocks[0]) {
            return false;
        }
    }
    true
}

#[cfg(test)]
#[allow(clippy::approx_constant)]
mod tests {
    use super::*;
    use alloc::vec;
    use approx::assert_relative_eq;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_array_constructor() {
        let input = BlockData::new(2, 3, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);

        assert_eq!(input.get_type(), BlockDataType::Matrix);
        assert_eq!(
            input._data.row(0),
            DMatrix::<f64>::from_row_slice(1, 3, &[1.0, 2.0, 3.0])
        );
        assert_eq!(
            input._data.row(1),
            DMatrix::<f64>::from_row_slice(1, 3, &[4.0, 5.0, 6.0])
        );
    }

    #[test]
    fn test_scalar_constructor() {
        let input = BlockData::from_scalar(3.14159);

        assert_eq!(input.get_type(), BlockDataType::Scalar);
        assert_eq!(input.scalar(), 3.14159);
    }

    #[test]
    fn test_set_scalar() {
        let mut input = BlockData::from_scalar(3.14159);
        input.set_scalar(1.0);

        assert_eq!(input.get_type(), BlockDataType::Scalar);
        assert_eq!(input.scalar(), 1.0);
    }

    #[test]
    fn test_bool_constructor() {
        let block = BlockData::scalar_from_bool(false);
        assert!(!block.any());

        let block = BlockData::scalar_from_bool(true);
        assert!(block.all())
    }

    #[test]
    fn test_vector_constructor() {
        let block = BlockData::from_matrix(&[&[1., 2.], &[3., 4.], &[5., 6.]]);

        assert!(block.get_type() == BlockDataType::Matrix);
        assert!(block.nrows() == 3);
        assert!(block.ncols() == 2);
        assert!(block.at_rc(0, 0) == 1.);
        assert!(block.at_rc(0, 1) == 2.);
        assert!(block.at_rc(1, 0) == 3.);
        assert!(block.at_rc(1, 1) == 4.);
        assert!(block.at_rc(2, 0) == 5.);
        assert!(block.at_rc(2, 1) == 6.);
    }

    #[test]
    fn test_set_bool() {
        let mut block = BlockData::scalar_from_bool(false);

        block.set_scalar_bool(true);
        assert!(block.all());

        block.set_scalar_bool(false);
        assert!(!block.any());
    }

    #[test]
    fn test_or() {
        let a = BlockData::from_vector(&[-1., 0., 1., 2.]);
        let b = BlockData::zeros_sizeof(&a);

        assert_eq!(a.or(&b), BlockData::from_vector(&[1.0, 0.0, 1.0, 1.0]));
    }

    #[test]
    fn test_and() {
        let a = BlockData::from_vector(&[-1., 0., 1., 2.]);
        let b = BlockData::from_vector(&[0., 0., 1., -2.]);

        assert_eq!(a.and(&b), BlockData::from_vector(&[0.0, 0.0, 1.0, 1.0]));
    }

    #[test]
    fn test_eq() {
        let a = BlockData::from_vector(&[-1., 0., 1., 2.]);
        let b = BlockData::zeros_sizeof(&a);

        assert_eq!(a.eq(&b), BlockData::from_vector(&[0.0, 1.0, 0.0, 0.0]));
    }

    #[test]
    fn test_neq() {
        let a = BlockData::from_vector(&[-1., 0., 1., 2.]);
        let b = BlockData::zeros_sizeof(&a);

        assert_eq!(a.neq(&b), BlockData::from_vector(&[1.0, 0.0, 1.0, 1.0]));
    }

    #[test]
    fn test_gt() {
        let a = BlockData::from_vector(&[-1., 0., 1., 2.]);
        let b = BlockData::zeros_sizeof(&a);

        assert_eq!(a.gt(&b), BlockData::from_vector(&[0.0, 0.0, 1.0, 1.0]));
    }

    #[test]
    fn test_gte() {
        let a = BlockData::from_vector(&[-1., 0., 1., 2.]);
        let b = BlockData::zeros_sizeof(&a);

        assert_eq!(a.gte(&b), BlockData::from_vector(&[0.0, 1.0, 1.0, 1.0]));
    }

    #[test]
    fn test_lt() {
        let a = BlockData::from_vector(&[-1., 0., 1., 2.]);
        let b = BlockData::zeros_sizeof(&a);

        assert_eq!(a.lt(&b), BlockData::from_vector(&[1.0, 0.0, 0.0, 0.0]));
    }

    #[test]
    fn test_lte() {
        let a = BlockData::from_vector(&[-1., 0., 1., 2.]);
        let b = BlockData::zeros_sizeof(&a);

        assert_eq!(a.lte(&b), BlockData::from_vector(&[1.0, 1.0, 0.0, 0.0]));
    }

    #[test]
    fn test_ltz() {
        let a = BlockData::from_vector(&[-1., 0., 1., 2.]);
        assert_eq!(a.ltz(), BlockData::from_vector(&[1.0, 0.0, 0.0, 0.0]));
    }

    #[test]
    fn test_gtz() {
        let a = BlockData::from_vector(&[-1., 0., 1., 2.]);
        assert_eq!(a.gtz(), BlockData::from_vector(&[0.0, 0.0, 1.0, 1.0]));
    }

    #[test]
    fn test_ltez() {
        let a = BlockData::from_vector(&[-1., 0., 1., 2.]);
        assert_eq!(a.ltez(), BlockData::from_vector(&[1.0, 1.0, 0.0, 0.0]));
    }

    #[test]
    fn test_gtez() {
        let a = BlockData::from_vector(&[-1., 0., 1., 2.]);
        assert_eq!(a.gtez(), BlockData::from_vector(&[0.0, 1.0, 1.0, 1.0]));
    }

    #[test]
    fn test_not() {
        let a = BlockData::from_vector(&[-1., 0., 1., 2.]);
        assert_eq!(
            a.logical_not(),
            BlockData::from_vector(&[0.0, 1.0, 0.0, 0.0])
        );
    }

    #[test]
    fn test_maybe_reset() {
        let mut a = BlockData::from_vector(&[-1., 10., 20., 2.]);
        a.maybe_reset(&BlockData::from_vector(&[-1., 0., 1., 2.]));

        assert_eq!(a, BlockData::from_vector(&[0.0, 10.0, 0.0, 0.0]));

        // Can also reset a matrix with a scalar reset
        a.maybe_reset(&BlockData::from_scalar(1.0));
        assert_eq!(a, BlockData::from_vector(&[0.0, 0.0, 0.0, 0.0]));
    }

    #[test]
    fn test_powf() {
        let base = BlockData::from_vector(&[1.0, -2.0, 3.0]);
        let exponent = BlockData::from_vector(&[2.0, 3.0, 4.0]);

        assert_eq!(
            base.powf(&exponent),
            BlockData::from_vector(&[1.0, -8.0, 81.0])
        );
    }

    #[test]
    fn test_sign() {
        assert_eq!(
            BlockData::from_vector(&[-2., -1., 0., 1., 2.]).sign(),
            BlockData::from_vector(&[-1., -1., 0., 1., 1.]),
        );
    }

    #[test]
    fn test_boolean() {
        assert_eq!(
            BlockData::from_vector(&[-2., -1., 0., 1., 2.]).boolean(),
            BlockData::from_vector(&[1., 1., 0., 1., 1.]),
        );
    }

    #[test]
    fn test_all() {
        assert_eq!(BlockData::from_vector(&[-2., -1., 0., 1., 2.]).all(), false);
        assert_eq!(BlockData::from_vector(&[-2., -1., 1., 1., 2.]).all(), true);
    }
    #[test]
    fn test_any() {
        assert_eq!(BlockData::from_vector(&[-2., -1., 0., 1., 2.]).any(), true);
        assert_eq!(BlockData::from_vector(&[0., 0., 0., 0., 0.]).any(), false);
    }
    #[test]
    fn test_inverse() {
        let block = BlockData::new(2, 2, &[1., 2., 3., 4.]);
        // From numpy
        assert_eq!(
            block.inverse().unwrap(),
            BlockData::new(2, 2, &[-2., 1., 1.5, -0.5])
        );
    }
    #[test]
    fn test_dot() {
        let block1 = BlockData::new(1, 3, &[1.0, 2.0, 3.0]);
        let block2 = BlockData::new(1, 3, &[4.0, 1.0, 0.0]);
        // From numpy
        assert_eq!(block1.dot(&block2), BlockData::from_scalar(6.0));
    }
    #[test]
    fn test_determinant() {
        let block = BlockData::new(2, 2, &[1.0, 2.0, 3.0, 4.0]);
        // From numpy
        assert_eq!(block.determinant(), BlockData::from_scalar(-2.0));
    }
    #[test]
    fn test_cross() {
        let block1 = BlockData::new(1, 3, &[1.0, 0.0, 0.0]);
        let block2 = BlockData::new(1, 3, &[0.0, 1.0, 0.0]);
        // From numpy
        assert_eq!(
            block1.cross(&block2),
            BlockData::new(1, 3, &[0.0, 0.0, 1.0])
        );
    }
    #[test]
    fn test_transpose() {
        let block = BlockData::new(3, 2, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        // From numpy
        assert_eq!(
            block.transpose(),
            BlockData::new(2, 3, &[1.0, 3.0, 5.0, 2.0, 4.0, 6.0])
        );
    }
    #[test]
    fn test_sum() {
        let block = BlockData::new(3, 2, &[1.0, 2.0, 3.0, 4.0, 5.0, -6.0]);
        assert_eq!(block.sum(), 9.0);
    }
    #[test]
    fn test_mean() {
        let block = BlockData::new(3, 2, &[1.0, 2.0, 3.0, 4.0, 5.0, -6.0]);
        assert_eq!(block.mean(), 1.5);
    }
    #[test]
    fn test_median() {
        assert_eq!(
            BlockData::new(1, 7, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]).median(),
            4.0
        );
        assert_eq!(
            BlockData::new(1, 6, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).median(),
            3.5
        );
    }
    #[test]
    fn test_min() {
        assert_eq!(
            BlockData::new(1, 7, &[1.0, 2.0, 3.0, 4.0, -5.0, 6.0, 7.0]).min(),
            -5.0
        );
    }
    #[test]
    fn test_max() {
        assert_eq!(
            BlockData::new(1, 7, &[1.0, 2.0, 3.0, 4.0, -5.0, 6.0, 7.0]).max(),
            7.0
        );
    }
    #[test]
    fn test_component_div() {
        // Divide scalar by vector
        assert_eq!(
            BlockData::from_scalar(1.0) / BlockData::new(1, 2, &[2.0, -4.0]),
            BlockData::new(1, 2, &[0.5, -0.25])
        );
        // Divide vector by scalar
        assert_eq!(
            BlockData::new(1, 2, &[2.0, -4.0]) / BlockData::from_scalar(2.0),
            BlockData::new(1, 2, &[1.0, -2.0])
        );
        // Divide vector by vector
        assert_eq!(
            BlockData::new(1, 2, &[2.0, -4.0]) / BlockData::new(1, 2, &[4.0, -0.5]),
            BlockData::new(1, 2, &[0.5, 8.0])
        );
    }
    #[test]
    fn test_component_mul() {
        // scalar by vector
        assert_eq!(
            BlockData::from_scalar(2.0).component_mul(&BlockData::new(1, 2, &[2.0, -4.0])),
            BlockData::new(1, 2, &[4.0, -8.0])
        );
        // flip order
        assert_eq!(
            BlockData::new(1, 2, &[2.0, -4.0]).component_mul(&BlockData::from_scalar(2.0)),
            BlockData::new(1, 2, &[4.0, -8.0])
        );
        // vector by scalar
        assert_eq!(
            BlockData::new(1, 2, &[2.0, -4.0]).component_mul(&BlockData::from_scalar(2.0)),
            BlockData::new(1, 2, &[4.0, -8.0])
        );
        // vector by vector
        assert_eq!(
            BlockData::new(1, 2, &[2.0, -4.0]).component_mul(&BlockData::new(1, 2, &[4.0, -0.5])),
            BlockData::new(1, 2, &[8.0, 2.0])
        );
    }
    #[test]
    fn test_component_add() {
        // scalar by vector
        assert_eq!(
            BlockData::from_scalar(2.0) + &BlockData::new(1, 2, &[2.0, -4.0]),
            BlockData::new(1, 2, &[4.0, -2.0])
        );
        // vector by scalar
        assert_eq!(
            BlockData::new(1, 2, &[2.0, -4.0]) + &BlockData::from_scalar(2.0),
            BlockData::new(1, 2, &[4.0, -2.0])
        );
        // vector by vector
        assert_eq!(
            BlockData::new(1, 2, &[2.0, -4.0]) + &BlockData::new(1, 2, &[4.0, -0.5]),
            BlockData::new(1, 2, &[6.0, -4.5])
        );
        // vector by scalar, in place
        let mut b = BlockData::new(1, 2, &[2.0, -4.0]);
        b += &BlockData::from_scalar(2.0);
        assert_eq!(b, BlockData::new(1, 2, &[4.0, -2.0]));
    }
    #[test]
    fn test_component_sub() {
        // scalar by vector
        assert_eq!(
            BlockData::from_scalar(2.0) - &BlockData::new(1, 2, &[2.0, -4.0]),
            BlockData::new(1, 2, &[0.0, 6.0])
        );
        // vector by scalar
        assert_eq!(
            BlockData::new(1, 2, &[2.0, -4.0]) - &BlockData::from_scalar(2.0),
            BlockData::new(1, 2, &[0.0, -6.0])
        );
        // vector by vector
        assert_eq!(
            BlockData::new(1, 2, &[2.0, -4.0]) - &BlockData::new(1, 2, &[4.0, -0.5]),
            BlockData::new(1, 2, &[-2.0, -3.5])
        );
        // vector by scalar, in place
        let mut b = BlockData::new(1, 2, &[2.0, -4.0]);
        b -= &BlockData::from_scalar(2.0);
        assert_eq!(b, BlockData::new(1, 2, &[0.0, -6.0]));
    }

    #[test]
    fn test_component_bitnot() {
        // Test data with positive and negative numbers
        let block = BlockData::from_vector(&[42.0, -3.5, 0.0, -1.0]);
        let expected = BlockData::from_vector(&[-43.0, 2.0, -1.0, 0.0]);

        // Test the function with the test data
        assert_eq!(block.component_bitnot(), expected);
    }

    #[test]
    fn test_bitand() {
        let a = BlockData::from_vector(&[1., 2., 3.]);
        let b = BlockData::from_vector(&[1., 1., 2.]);
        let c = a & b;

        let expected = BlockData::from_vector(&[1.0, 0.0, 2.0]);
        assert_eq!(c, expected);
    }

    #[test]
    fn test_bitor() {
        let a = BlockData::from_vector(&[1., 2., 3.]);
        let b = BlockData::from_vector(&[2., 3., 3.]);
        let c = a | b;

        let expected = BlockData::from_vector(&[3.0, 3.0, 3.0]);
        assert_eq!(c, expected);
    }

    #[test]
    fn test_bitxor() {
        let a = BlockData::from_vector(&[1., 2., 3.]);
        let b = BlockData::from_vector(&[2., 1., 0.]);
        let c = a ^ b;

        let expected = BlockData::from_vector(&[3.0, 3.0, 3.0]);
        assert_eq!(c, expected);
    }

    #[test]
    fn test_shl() {
        let data = BlockData::from_vector(&[1., 2., 3.]);
        let shifted = data << 1;

        let expected = BlockData::from_vector(&[2.0, 4.0, 6.0]);
        assert_eq!(shifted, expected);
    }

    #[test]
    fn test_shr() {
        let data = BlockData::from_vector(&[1., 2., 4.]);
        let shifted = data >> 1;

        let expected = BlockData::from_vector(&[0.0, 1.0, 2.0]);
        assert_eq!(shifted, expected);
    }

    #[test]
    fn test_fix_non_finite() {
        let mut data = BlockData::from_row_slice(
            1,
            6,
            &[1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 5.0, 6.0],
        );

        data.fix_non_finite();

        assert_eq!(
            data,
            BlockData::from_row_slice(1, 6, &[1.0, 0.0, 0.0, 0.0, 5.0, 6.0])
        );
    }

    #[test]
    fn test_vector_magnitude() {
        let data = BlockData::from_matrix(&[&[0., 1., 2.], &[3., 4., 5.], &[6., 7., 8.]]);

        assert_relative_eq!(
            data.vector_magnitude(),
            BlockData::from_row_slice(1, 3, &[6.7, 8.1, 9.6]),
            max_relative = 0.1
        );

        assert_relative_eq!(
            data.vector_magnitude_rows(),
            BlockData::from_row_slice(1, 3, &[2.2, 7.1, 12.2]),
            max_relative = 0.1
        );
    }

    #[test]
    fn test_stringify() {
        let scalar_data = BlockData::from_scalar(3.14159);
        let arr_data = BlockData::from_vector(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let matrix_data = BlockData::from_row_slice(3, 2, &[1., 2., 3., 4., 5., 6.]);
        let bytes_data = BlockData::from_bytes(&[1, 2, 3, 4, 5, 6]);
        let str_data = BlockData::from_bytes(b"Hello, world!");

        assert_eq!(scalar_data.stringify(), "3.14159");
        assert_eq!(arr_data.stringify(), "[[1.0,2.0,3.0,4.0,5.0,6.0]]");
        assert_eq!(matrix_data.stringify(), "[[1.0,2.0],[3.0,4.0],[5.0,6.0]]");
        assert_eq!(bytes_data.stringify(), "[1,2,3,4,5,6]");
        assert_eq!(
            str_data.stringify(),
            "[72,101,108,108,111,44,32,119,111,114,108,100,33]"
        );
    }

    #[test]
    fn test_raw_string() {
        let scalar_data = BlockData::from_scalar(3.14159);
        let arr_data = BlockData::from_vector(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let matrix_data = BlockData::from_row_slice(3, 2, &[1., 2., 3., 4., 5., 6.]);

        let non_std_bytes = vec![1, 2, 3, 4, 5, 6];
        let bytes_data = BlockData::from_bytes(&non_std_bytes);
        let str_data = BlockData::from_bytes(b"Hello, world!");

        assert_eq!(scalar_data.raw_string(), "3.14159");
        assert_eq!(arr_data.raw_string(), "[[1.0,2.0,3.0,4.0,5.0,6.0]]");
        assert_eq!(matrix_data.raw_string(), "[[1.0,2.0],[3.0,4.0],[5.0,6.0]]");
        assert_eq!(
            bytes_data.raw_string(),
            String::from_utf8(non_std_bytes).unwrap()
        );
        assert_eq!(str_data.raw_string(), "Hello, world!");
    }

    #[test]
    fn test_argmax() {
        let data = BlockData::from_vector(&[3.0, 1.0, 2.0, 4.0, 5.0]);
        assert_eq!(data.argmax(), 4.0);

        // | 3 1 2 |
        // | 6 5 4 |  linear index of the max is "1.0"
        let data_matrix = BlockData::from_row_slice(2, 3, &[3.0, 1.0, 2.0, 6.0, 5.0, 4.0]);
        assert_eq!(data_matrix.argmax(), 1.0);
    }

    #[test]
    fn test_argmin() {
        let data = BlockData::from_vector(&[3.0, 1.0, 2.0, 4.0, 5.0]);
        assert_eq!(data.argmin(), 1.0);

        let data_matrix = BlockData::from_row_slice(2, 3, &[3.0, 1.0, 2.0, 4.0, 5.0, 6.0]);
        assert_eq!(data_matrix.argmin(), 2.0);
    }

    #[test]
    fn test_block_data_scalar_read_write_traits() {
        let mut data = BlockData::from_scalar(1.0);
        {
            let data_read = &data;
            assert_eq!(data_read.get_scalar(), 1.0);
        }

        {
            let mut data_write = &mut data;
            data_write.set_scalar_value(2.0);
        }
        assert_eq!(data.get_type(), BlockDataType::Scalar);

        {
            let data_read = &data;
            assert_eq!(data_read.get_scalar(), 2.0);
        }
    }

    #[test]
    fn test_block_data_matrix_read_write_traits() {
        let nrows = 1;
        let ncols = 3;
        let slice_data = [1.0, 2.0, 3.0];
        let mut data = BlockData::from_row_slice(nrows, ncols, &slice_data);
        let data_read = &data;
        {
            assert_eq!(
                data_read.get_matrix(),
                (nrows, ncols, slice_data.as_slice())
            );
        }

        let nrows = 2;
        let ncols = 2;
        let slice_data = [4.0, 6.0, 5.0, 7.0];
        {
            let mut data_write = &mut data;
            data_write.set_matrix_value(nrows, ncols, slice_data.as_slice());
        }
        assert_eq!(data.get_type(), BlockDataType::Matrix);

        {
            let data_read = &data;
            let (nrows, ncols, _read_slice) = data_read.get_matrix();
            assert_eq!(
                data_read.get_matrix(),
                (nrows, ncols, slice_data.as_slice())
            );
        }
    }

    #[test]
    fn test_sorted() {
        let data = BlockData::from_vector(&[3.0, 1.0, 2.0, 4.0, 5.0]);
        let sorted = data.sorted(true);
        assert_eq!(sorted, BlockData::from_vector(&[1.0, 2.0, 3.0, 4.0, 5.0]));

        let sorted = data.sorted(false);
        assert_eq!(sorted, BlockData::from_vector(&[5.0, 4.0, 3.0, 2.0, 1.0]));

        let data_matrix = BlockData::from_row_slice(2, 3, &[3.0, 1.0, 2.0, 4.0, 5.0, 6.0]);
        let sorted = data_matrix.sorted(true);
        assert_eq!(
            sorted,
            BlockData::from_vector(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0])
        );
    }
}
