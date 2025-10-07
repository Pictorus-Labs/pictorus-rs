//! Traits to attempt to bridge tuples of a single type with fixed-size arrays of that type.

use crate::{Pass, Scalar};

pub trait TupleEquivalent<T: Scalar, const N: usize>: Sized {
    type TupleEquivalent: Sized + Pass;

    fn into_tuple(self) -> Self::TupleEquivalent;
}

/// Generates impls for Array to Tuple conversions for sizes 1 to 12.
/// Example expansion:
/// ```rust ignore
/// impl<T: Scalar> TupleEquivalent<T, 1> for [T; 1] {
///     type TupleEquivalent = T;
///
///     fn into_tuple(self) -> Self::TupleEquivalent {
///         self[0]
///     }
/// }
///
/// impl<T: Scalar> TupleEquivalent<T, 2> for [T; 2] {
///     type TupleEquivalent = (T, T);
///
///     fn into_tuple(self) -> Self::TupleEquivalent {
///         self.into()
///     }
/// }
/// ```
macro_rules! impl_tuple_array_interop {
    (1) => {
        impl<T: Scalar> TupleEquivalent<T, 1> for [T; 1] {
            type TupleEquivalent = T;

            fn into_tuple(self) -> Self::TupleEquivalent {
                self[0]
            }
        }
    };

    ($n:expr, $($t:ident),+) => {
        impl<T: Scalar> TupleEquivalent<T, $n> for [T; $n] {
            type TupleEquivalent = ($($t,)+);

            fn into_tuple(self) -> Self::TupleEquivalent {
                self.into()
            }
        }
    };
}

// Currently implement up to 12-tuples/arrays since we are using
// Default and From: https://doc.rust-lang.org/std/primitive.tuple.html
impl_tuple_array_interop!(1);
impl_tuple_array_interop!(2, T, T);
impl_tuple_array_interop!(3, T, T, T);
impl_tuple_array_interop!(4, T, T, T, T);
impl_tuple_array_interop!(5, T, T, T, T, T);
impl_tuple_array_interop!(6, T, T, T, T, T, T);
impl_tuple_array_interop!(7, T, T, T, T, T, T, T);
impl_tuple_array_interop!(8, T, T, T, T, T, T, T, T);
impl_tuple_array_interop!(9, T, T, T, T, T, T, T, T, T);
impl_tuple_array_interop!(10, T, T, T, T, T, T, T, T, T, T);
impl_tuple_array_interop!(11, T, T, T, T, T, T, T, T, T, T, T);
impl_tuple_array_interop!(12, T, T, T, T, T, T, T, T, T, T, T, T);
