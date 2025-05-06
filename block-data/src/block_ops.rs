//! This file contains all the basic mathematic operations (Add/Multiply/Divide/etc) we need
//! specific BlockData implementations for.
//!
//! Eventually these might all become macro-generated.

use approx::{AbsDiffEq, RelativeEq};
use core::ops::{
    Add, AddAssign, BitAnd, BitOr, BitXor, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Shl,
    Shr, Sub, SubAssign,
};

use nalgebra::DMatrix;

use crate::BlockData;
use crate::BlockTypeRelationship;

impl Sub for &BlockData {
    type Output = BlockData;
    fn sub(self, rhs: Self) -> Self::Output {
        match self.compare(rhs) {
            BlockTypeRelationship::SameSizes => {
                BlockData::from_data(self.get_data() - rhs.get_data(), self.get_type())
            }
            BlockTypeRelationship::FirstIsScalar => BlockData::from_data(
                BlockData::scalar_sizeof(self.scalar(), rhs).get_data() - rhs.get_data(),
                rhs.get_type(),
            ),
            BlockTypeRelationship::SecondIsScalar => BlockData::from_data(
                self.get_data() - BlockData::scalar_sizeof(rhs.scalar(), self).get_data(),
                self.get_type(),
            ),
            _ => panic!("Cannot add unless sizes match or one is scalar."),
        }
    }
}

impl Sub<&BlockData> for BlockData {
    type Output = Self;
    fn sub(self, rhs: &BlockData) -> Self::Output {
        &self - rhs
    }
}

impl Sub<BlockData> for &BlockData {
    type Output = BlockData;
    fn sub(self, rhs: BlockData) -> Self::Output {
        self - &rhs
    }
}

impl SubAssign<&BlockData> for BlockData {
    fn sub_assign(&mut self, rhs: &BlockData) {
        match self.compare(rhs) {
            BlockTypeRelationship::SameSizes => self.set_data(self.get_data() - rhs.get_data()),
            BlockTypeRelationship::FirstIsScalar => {
                self.set_data(
                    BlockData::scalar_sizeof(self.scalar(), rhs).get_data() - rhs.get_data(),
                );
                self.set_type(rhs.get_type());
            }
            BlockTypeRelationship::SecondIsScalar => self.set_data(
                self.get_data() - BlockData::scalar_sizeof(rhs.scalar(), self).get_data(),
            ),
            _ => panic!("Cannot add unless sizes match or one is scalar."),
        }
    }
}

impl Add for &BlockData {
    type Output = BlockData;
    fn add(self, rhs: &BlockData) -> Self::Output {
        match self.compare(rhs) {
            BlockTypeRelationship::SameSizes => {
                BlockData::from_data(self.get_data() + rhs.get_data(), self.get_type())
            }
            BlockTypeRelationship::FirstIsScalar => BlockData::from_data(
                BlockData::scalar_sizeof(self.scalar(), rhs).get_data() + rhs.get_data(),
                rhs.get_type(),
            ),
            BlockTypeRelationship::SecondIsScalar => BlockData::from_data(
                BlockData::scalar_sizeof(rhs.scalar(), self).get_data() + self.get_data(),
                self.get_type(),
            ),
            _ => panic!("Cannot add unless sizes match or one is scalar."),
        }
    }
}

impl Add<&BlockData> for BlockData {
    type Output = BlockData;
    fn add(self, rhs: &BlockData) -> Self::Output {
        &self + rhs
    }
}

impl AddAssign<&BlockData> for BlockData {
    fn add_assign(&mut self, rhs: &BlockData) {
        match self.compare(rhs) {
            BlockTypeRelationship::SameSizes => self.set_data(self.get_data() + rhs.get_data()),
            BlockTypeRelationship::FirstIsScalar => {
                self.set_data(
                    BlockData::scalar_sizeof(self.scalar(), rhs).get_data() + rhs.get_data(),
                );
                self.set_type(rhs.get_type());
            }
            BlockTypeRelationship::SecondIsScalar => self.set_data(
                self.get_data() + BlockData::scalar_sizeof(rhs.scalar(), self).get_data(),
            ),
            _ => panic!("Cannot add unless sizes match or one is scalar."),
        }
    }
}

impl Mul for &BlockData {
    type Output = BlockData;
    fn mul(self, rhs: &BlockData) -> Self::Output {
        BlockData::from_data(self.get_data() * rhs.get_data(), self.get_type())
    }
}

impl Mul<f64> for &BlockData {
    type Output = BlockData;
    fn mul(self, rhs: f64) -> Self::Output {
        BlockData::from_data(self.get_data() * rhs, self.get_type())
    }
}

impl Mul<f64> for BlockData {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self::Output {
        &self * rhs
    }
}

impl Mul<&BlockData> for f64 {
    type Output = BlockData;
    fn mul(self, rhs: &BlockData) -> Self::Output {
        rhs * self
    }
}

impl Mul<BlockData> for f64 {
    type Output = BlockData;
    fn mul(self, rhs: BlockData) -> Self::Output {
        &rhs * self
    }
}

impl MulAssign<&BlockData> for BlockData {
    fn mul_assign(&mut self, rhs: &BlockData) {
        self.set_data(self.get_data() * rhs.get_data());
    }
}

impl Index<usize> for BlockData {
    type Output = f64;
    fn index(&self, index: usize) -> &Self::Output {
        self.ref_at(index)
    }
}

impl IndexMut<usize> for BlockData {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.ref_at_mut(index)
    }
}

impl Div<f64> for BlockData {
    type Output = Self;
    fn div(self, rhs: f64) -> Self::Output {
        Self::from_data(self.get_data() / rhs, self.get_type())
    }
}

impl Div<f64> for &BlockData {
    type Output = BlockData;
    fn div(self, rhs: f64) -> Self::Output {
        BlockData::from_data(self.get_data() / rhs, self.get_type())
    }
}

impl DivAssign<f64> for BlockData {
    fn div_assign(&mut self, rhs: f64) {
        self.set_data(self.get_data() / rhs);
    }
}

impl Div for &BlockData {
    type Output = BlockData;
    fn div(self, rhs: &BlockData) -> Self::Output {
        match self.compare(rhs) {
            BlockTypeRelationship::SameSizes => BlockData::from_data(
                self.get_data().component_div(rhs.get_data()),
                self.get_type(),
            ),
            BlockTypeRelationship::FirstIsScalar => BlockData::from_data(
                BlockData::scalar_sizeof(self.scalar(), rhs)
                    .get_data()
                    .component_div(rhs.get_data()),
                rhs.get_type(),
            ),
            BlockTypeRelationship::SecondIsScalar => BlockData::from_data(
                self.get_data()
                    .component_div(BlockData::scalar_sizeof(rhs.scalar(), self).get_data()),
                self.get_type(),
            ),
            _ => panic!("Cannot add unless sizes match or one is scalar."),
        }
    }
}
impl Div for BlockData {
    type Output = BlockData;
    fn div(self, rhs: BlockData) -> Self::Output {
        &self / &rhs
    }
}

impl DivAssign<&BlockData> for BlockData {
    fn div_assign(&mut self, rhs: &BlockData) {
        match self.compare(rhs) {
            BlockTypeRelationship::SameSizes => {
                self.set_data(self.get_data().component_div(rhs.get_data()))
            }
            BlockTypeRelationship::FirstIsScalar => self.set_data(
                BlockData::scalar_sizeof(self.scalar(), rhs)
                    .get_data()
                    .component_div(rhs.get_data()),
            ),
            BlockTypeRelationship::SecondIsScalar => self.set_data(
                self.get_data()
                    .component_div(BlockData::scalar_sizeof(rhs.scalar(), self).get_data()),
            ),
            _ => panic!("Cannot add unless sizes match or one is scalar."),
        }
    }
}

impl BitAnd for &BlockData {
    type Output = BlockData;
    fn bitand(self, rhs: &BlockData) -> Self::Output {
        match self.compare(rhs) {
            BlockTypeRelationship::SameSizes => self.component_bitand(rhs),
            BlockTypeRelationship::FirstIsScalar => {
                BlockData::scalar_sizeof(self.scalar(), rhs).component_bitand(rhs)
            }
            BlockTypeRelationship::SecondIsScalar => {
                self.component_bitand(&BlockData::scalar_sizeof(rhs.scalar(), self))
            }
            _ => panic!("Cannot AND unless sizes match or one is scalar."),
        }
    }
}

impl BitAnd for BlockData {
    type Output = BlockData;
    fn bitand(self, rhs: BlockData) -> Self::Output {
        &self & &rhs
    }
}

impl BitOr for &BlockData {
    type Output = BlockData;
    fn bitor(self, rhs: &BlockData) -> Self::Output {
        match self.compare(rhs) {
            BlockTypeRelationship::SameSizes => self.component_bitor(rhs),
            BlockTypeRelationship::FirstIsScalar => {
                BlockData::scalar_sizeof(self.scalar(), rhs).component_bitor(rhs)
            }
            BlockTypeRelationship::SecondIsScalar => {
                self.component_bitor(&BlockData::scalar_sizeof(rhs.scalar(), self))
            }
            _ => panic!("Cannot OR unless sizes match or one is scalar."),
        }
    }
}

impl BitOr for BlockData {
    type Output = BlockData;
    fn bitor(self, rhs: BlockData) -> Self::Output {
        &self | &rhs
    }
}

impl BitXor for &BlockData {
    type Output = BlockData;
    fn bitxor(self, rhs: &BlockData) -> Self::Output {
        match self.compare(rhs) {
            BlockTypeRelationship::SameSizes => self.component_bitxor(rhs),
            BlockTypeRelationship::FirstIsScalar => {
                BlockData::scalar_sizeof(self.scalar(), rhs).component_bitxor(rhs)
            }
            BlockTypeRelationship::SecondIsScalar => {
                self.component_bitxor(&BlockData::scalar_sizeof(rhs.scalar(), self))
            }
            _ => panic!("Cannot XOR unless sizes match or one is scalar."),
        }
    }
}

impl BitXor for BlockData {
    type Output = BlockData;
    fn bitxor(self, rhs: BlockData) -> Self::Output {
        &self ^ &rhs
    }
}

impl Shl<i32> for &BlockData {
    type Output = BlockData;
    fn shl(self, rhs: i32) -> Self::Output {
        self.component_lshift(rhs)
    }
}

impl Shl<i32> for BlockData {
    type Output = BlockData;
    fn shl(self, rhs: i32) -> Self::Output {
        &self << rhs
    }
}

impl Shr<i32> for &BlockData {
    type Output = BlockData;
    fn shr(self, rhs: i32) -> Self::Output {
        self.component_rshift(rhs)
    }
}

impl Shr<i32> for BlockData {
    type Output = BlockData;
    fn shr(self, rhs: i32) -> Self::Output {
        &self >> rhs
    }
}

impl AbsDiffEq for BlockData {
    type Epsilon = f64;

    fn default_epsilon() -> Self::Epsilon {
        DMatrix::<f64>::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.get_data().abs_diff_eq(other.get_data(), epsilon)
    }

    fn abs_diff_ne(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.get_data().abs_diff_ne(other.get_data(), epsilon)
    }
}

impl RelativeEq for BlockData {
    fn default_max_relative() -> Self::Epsilon {
        DMatrix::<f64>::default_max_relative()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        self.get_data()
            .relative_eq(other.get_data(), epsilon, max_relative)
    }

    fn relative_ne(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        self.get_data()
            .relative_ne(other.get_data(), epsilon, max_relative)
    }
}
