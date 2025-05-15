//! Traits to attempt to bridge tuples of a single type with fixed-size arrays of that type.

use crate::{Pass, Scalar};

pub trait TupleEquivalent<T: Scalar, const N: usize>: Sized {
    type TupleEquivalent: Sized + Default + Pass;

    fn into_tuple(self) -> Self::TupleEquivalent;
}

impl<T: Scalar> TupleEquivalent<T, 1> for [T; 1] {
    type TupleEquivalent = T;

    fn into_tuple(self) -> Self::TupleEquivalent {
        self[0]
    }
}

impl<T: Scalar> TupleEquivalent<T, 2> for [T; 2] {
    type TupleEquivalent = (T, T);

    fn into_tuple(self) -> Self::TupleEquivalent {
        self.into()
    }
}

impl<T: Scalar> TupleEquivalent<T, 3> for [T; 3] {
    type TupleEquivalent = (T, T, T);

    fn into_tuple(self) -> Self::TupleEquivalent {
        self.into()
    }
}

impl<T: Scalar> TupleEquivalent<T, 4> for [T; 4] {
    type TupleEquivalent = (T, T, T, T);

    fn into_tuple(self) -> Self::TupleEquivalent {
        self.into()
    }
}

impl<T: Scalar> TupleEquivalent<T, 5> for [T; 5] {
    type TupleEquivalent = (T, T, T, T, T);

    fn into_tuple(self) -> Self::TupleEquivalent {
        self.into()
    }
}

impl<T: Scalar> TupleEquivalent<T, 6> for [T; 6] {
    type TupleEquivalent = (T, T, T, T, T, T);

    fn into_tuple(self) -> Self::TupleEquivalent {
        self.into()
    }
}

impl<T: Scalar> TupleEquivalent<T, 7> for [T; 7] {
    type TupleEquivalent = (T, T, T, T, T, T, T);

    fn into_tuple(self) -> Self::TupleEquivalent {
        self.into()
    }
}

impl<T: Scalar> TupleEquivalent<T, 8> for [T; 8] {
    type TupleEquivalent = (T, T, T, T, T, T, T, T);

    fn into_tuple(self) -> Self::TupleEquivalent {
        self.into()
    }
}
