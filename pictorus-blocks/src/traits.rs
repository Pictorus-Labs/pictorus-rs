//! A collection of traits that are used in the corelib-blocks library
extern crate alloc;
use alloc::vec::Vec;
use core::time::Duration;
use nalgebra::{ComplexField, RealField, SimdPartialOrd};
#[cfg(feature = "tricore")]
use pictorus_traits::FlattenSlice as _;
use pictorus_traits::{ByteSliceSignal, Matrix, Pass, PassBy};

pub mod serialize;
pub use serialize::Serialize;

/// A re-export of the pictorus_traits::Scalar trait to allow for easier blanket implementations
pub trait Scalar:
    pictorus_traits::Scalar
    + for<'a> Pass<By<'a> = Self>
    + PartialEq
    + nalgebra::Scalar
    + SimdPartialOrd
{
    /// Returns true if the scalar is truthy
    /// Truthiness is defined as not equal to zero
    fn is_truthy(&self) -> bool;
}
impl Scalar for bool {
    fn is_truthy(&self) -> bool {
        *self
    }
}
impl Scalar for u8 {
    fn is_truthy(&self) -> bool {
        *self != 0
    }
}
impl Scalar for i8 {
    fn is_truthy(&self) -> bool {
        *self != 0
    }
}
impl Scalar for u16 {
    fn is_truthy(&self) -> bool {
        *self != 0
    }
}
impl Scalar for i16 {
    fn is_truthy(&self) -> bool {
        *self != 0
    }
}
impl Scalar for u32 {
    fn is_truthy(&self) -> bool {
        *self != 0
    }
}
impl Scalar for i32 {
    fn is_truthy(&self) -> bool {
        *self != 0
    }
}
impl Scalar for f32 {
    fn is_truthy(&self) -> bool {
        *self != 0.0
    }
}
impl Scalar for f64 {
    fn is_truthy(&self) -> bool {
        *self != 0.0
    }
}

pub trait Float: Scalar + num_traits::Float + ComplexField
where
    Self: RealField<RealField = Self>,
{
    const EPSILON: Self;
    const PI: Self;
    const TAU: Self;

    fn from_duration(duration: Duration) -> Self;
}

impl Float for f32 {
    const EPSILON: Self = f32::EPSILON;
    const PI: Self = core::f32::consts::PI;
    const TAU: Self = core::f32::consts::TAU;

    fn from_duration(duration: Duration) -> Self {
        duration.as_secs_f32()
    }
}

impl Float for f64 {
    const EPSILON: Self = f64::EPSILON;
    const PI: Self = core::f64::consts::PI;
    const TAU: Self = core::f64::consts::TAU;

    fn from_duration(duration: Duration) -> Self {
        duration.as_secs_f64()
    }
}

pub trait DefaultStorage: Pass + CopyInto<Self::Storage> {
    type Storage;

    fn default_storage() -> Self::Storage;

    fn from_storage(storage: &Self::Storage) -> PassBy<'_, Self>;
}

impl<T: Scalar> DefaultStorage for T {
    type Storage = T;

    fn default_storage() -> Self::Storage {
        T::default()
    }

    fn from_storage(storage: &Self::Storage) -> PassBy<'_, Self> {
        storage.as_by()
    }
}

impl<T: Scalar, const NCOLS: usize, const NRWOS: usize> DefaultStorage for Matrix<NRWOS, NCOLS, T> {
    type Storage = Matrix<NRWOS, NCOLS, T>;

    fn default_storage() -> Self::Storage {
        Matrix::zeroed()
    }

    fn from_storage(storage: &Self::Storage) -> PassBy<'_, Self> {
        storage.as_by()
    }
}

impl DefaultStorage for ByteSliceSignal {
    type Storage = Vec<u8>;

    fn default_storage() -> Self::Storage {
        Vec::new()
    }

    fn from_storage(storage: &Self::Storage) -> PassBy<'_, Self> {
        storage.as_slice()
    }
}

/// A trait that allows for copying a PassBy of a type into an instance of that type
/// or in the case of scalar into a matrix of that scalar (every element is set to the scalar)
pub trait CopyInto<T>: Pass {
    fn copy_into(source: PassBy<Self>, dest: &mut T);
}

impl<S> CopyInto<S> for S
where
    S: Scalar,
{
    fn copy_into(source: PassBy<Self>, dest: &mut S) {
        *dest = source;
    }
}

impl<const NROWS: usize, const NCOLS: usize, S> CopyInto<Matrix<NROWS, NCOLS, S>> for S
where
    S: Scalar,
{
    fn copy_into(source: PassBy<Self>, dest: &mut Matrix<NROWS, NCOLS, S>) {
        dest.data
            .as_flattened_mut()
            .iter_mut()
            .for_each(|x| *x = source);
    }
}

impl<const NROWS: usize, const NCOLS: usize, S> CopyInto<Matrix<NROWS, NCOLS, S>>
    for Matrix<NROWS, NCOLS, S>
where
    S: Scalar,
{
    fn copy_into(source: PassBy<Self>, dest: &mut Matrix<NROWS, NCOLS, S>) {
        dest.data = source.data;
    }
}

impl CopyInto<Vec<u8>> for ByteSliceSignal {
    fn copy_into(source: PassBy<Self>, dest: &mut Vec<u8>) {
        dest.clear();
        dest.extend_from_slice(source);
    }
}

/// A recursive trait that allows one to refer to the output type relative to an input tuple
/// that may contain all scalars, all matrices (of the same size), or a mix of scalars and
/// all matrices (of the same size). The output will be a scalar for all scalar inputs, a matrix
/// for all matrix inputs, or a matrix for a mix of scalar and matrix inputs.
///
/// This is not a universal rule (see the product block) but is a common pattern for blocks
/// in our library.
pub trait SizePromotion<RHS> {
    type Output: Pass + Default;
}

/// A scalar and a scalar will return a scalar
impl<S: Scalar> SizePromotion<S> for S {
    type Output = S;
}

/// A matrix and a scalar will return a matrix
impl<S: Scalar, const NROWS: usize, const NCOLS: usize> SizePromotion<Matrix<NROWS, NCOLS, S>>
    for S
{
    type Output = Matrix<NROWS, NCOLS, S>;
}

/// A scalar and a matrix will return a matrix
impl<S: Scalar, const NROWS: usize, const NCOLS: usize> SizePromotion<S>
    for Matrix<NROWS, NCOLS, S>
{
    type Output = Matrix<NROWS, NCOLS, S>;
}

/// A Matrix and a Matrix will return a Matrix
impl<const NROWS: usize, const NCOLS: usize, S: Scalar> SizePromotion<Matrix<NROWS, NCOLS, S>>
    for Matrix<NROWS, NCOLS, S>
{
    type Output = Matrix<NROWS, NCOLS, S>;
}

/// Recursive Definition for 3 inputs
impl<A, B, C> SizePromotion<(B, C)> for A
where
    B: SizePromotion<C>,
    A: SizePromotion<B::Output>,
{
    type Output = <A as SizePromotion<B::Output>>::Output;
}

/// Recursive Definition for 4 inputs
impl<A, B, C, D> SizePromotion<(B, C, D)> for A
where
    B: SizePromotion<(C, D)>,
    A: SizePromotion<B::Output>,
{
    type Output = <A as SizePromotion<B::Output>>::Output;
}

/// Recursive Definition for 5 inputs
impl<A, B, C, D, E> SizePromotion<(B, C, D, E)> for A
where
    B: SizePromotion<(C, D, E)>,
    A: SizePromotion<B::Output>,
{
    type Output = <A as SizePromotion<B::Output>>::Output;
}

/// Recursive Definition for 6 inputs
impl<A, B, C, D, E, F> SizePromotion<(B, C, D, E, F)> for A
where
    B: SizePromotion<(C, D, E, F)>,
    A: SizePromotion<B::Output>,
{
    type Output = <A as SizePromotion<B::Output>>::Output;
}

/// Recursive Definition for 7 inputs
impl<A, B, C, D, E, F, G> SizePromotion<(B, C, D, E, F, G)> for A
where
    B: SizePromotion<(C, D, E, F, G)>,
    A: SizePromotion<B::Output>,
{
    type Output = <A as SizePromotion<B::Output>>::Output;
}

/// Recursive Definition for 8 inputs
impl<A, B, C, D, E, F, G, H> SizePromotion<(B, C, D, E, F, G, H)> for A
where
    B: SizePromotion<(C, D, E, F, G, H)>,
    A: SizePromotion<B::Output>,
{
    type Output = <A as SizePromotion<B::Output>>::Output;
}

/// Helper functions for working with Matrix structs
pub trait MatrixOps<const NROWS: usize, const NCOLS: usize, T: Scalar> {
    /// Iterates over all elements in a matrix and applies the provided function. The function
    /// inputs are the current value of the element being indexed (T), the column index (usize),
    /// and the row index (usize) in that order.
    fn for_each(&self, f: impl FnMut(T, usize, usize));

    /// Creates a matrix with all elements set to the value.
    fn from_element(value: T) -> Self;

    /// Applies a function to an existing matrix and creates a new matrix. The function inputs are the
    /// current value of the element being indexed (T), the column index (usize), and the row index (usize)
    /// in that order. Useful for mapping a matrix with a transformation applied to each element into a new matrix.
    fn map_collect(&self, f: impl FnMut(T, usize, usize) -> T) -> Self;
}

impl<const NROWS: usize, const NCOLS: usize, T> MatrixOps<NROWS, NCOLS, T>
    for Matrix<NROWS, NCOLS, T>
where
    T: Scalar,
{
    /// Iterates over all elements in the matrix and applies the provided function
    /// to each element
    fn for_each(&self, mut f: impl FnMut(T, usize, usize)) {
        self.data.iter().enumerate().for_each(|(c, col)| {
            col.iter()
                .enumerate()
                .for_each(|(r, &value)| f(value, c, r));
        });
    }

    fn map_collect(&self, mut f: impl FnMut(T, usize, usize) -> T) -> Self {
        let mut output = Self::zeroed();
        self.for_each(|v, c, r| {
            output.data[c][r] = f(v, c, r);
        });
        output
    }

    /// Creates a matrix with all elements set to the value
    fn from_element(value: T) -> Self {
        let mut mat = Self::zeroed();
        T::copy_into(value, &mut mat);
        mat
    }
}
/// The Apply and ApplyInto traits can be used in combination to easily define a block
/// that can accept a dynamic number of inputs as same-sized matrices or scalars, and output
/// a single value that is either a matrix or scalar.
///
/// To do this, you just need to define the ApplyInto trait for scalar -> scalar, scalar -> matrix,
/// and matrix -> matrix.
///
/// Note: This currently only works for blocks that require only their parameters and inputs to
/// determine the output. If a block requires additional context, you will need to create a bespoke
/// implementation.
pub trait ApplyInto<DEST: Pass, P>: Pass + Default {
    fn apply_into<'a>(
        input: PassBy<Self>,
        params: &P,
        dest: &'a mut Option<DEST>,
    ) -> PassBy<'a, DEST>;
}

pub trait Apply<P>: Pass + Default {
    type Output: Pass + Default;

    fn apply<'a>(
        input: PassBy<Self>,
        params: &P,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output>;
}

// Single scalar input
impl<S: Scalar, P> Apply<P> for S
where
    S: ApplyInto<S, P>,
{
    type Output = S;

    fn apply<'a>(
        input: PassBy<Self>,
        _params: &P,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        dest.insert(input).as_by()
    }
}

// Single Matrix input
impl<const R: usize, const C: usize, S: Scalar, P> Apply<P> for Matrix<R, C, S>
where
    Self: ApplyInto<Matrix<R, C, S>, P> + for<'a> Pass<By<'a> = &'a Self>,
{
    type Output = Matrix<R, C, S>;

    fn apply<'a>(
        input: PassBy<Self>,
        _params: &P,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        dest.insert(*input).as_by()
    }
}

impl<A, B, P> Apply<P> for (A, B)
where
    A: SizePromotion<B>,
    A: ApplyInto<A::Output, P>,
    B: ApplyInto<A::Output, P>,
{
    type Output = A::Output;

    fn apply<'a>(
        input: PassBy<Self>,
        params: &P,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (a, b) = input;
        A::apply_into(a, params, dest);
        B::apply_into(b, params, dest)
    }
}

impl<A, B, C, P> Apply<P> for (A, B, C)
where
    A: SizePromotion<(B, C)>,
    A: ApplyInto<A::Output, P>,
    B: ApplyInto<A::Output, P>,
    C: ApplyInto<A::Output, P>,
{
    type Output = A::Output;

    fn apply<'a>(
        input: PassBy<Self>,
        params: &P,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (a, b, c) = input;
        A::apply_into(a, params, dest);
        B::apply_into(b, params, dest);
        C::apply_into(c, params, dest)
    }
}

impl<A, B, C, D, P> Apply<P> for (A, B, C, D)
where
    A: SizePromotion<(B, C, D)>,
    A: ApplyInto<A::Output, P>,
    B: ApplyInto<A::Output, P>,
    C: ApplyInto<A::Output, P>,
    D: ApplyInto<A::Output, P>,
{
    type Output = A::Output;

    fn apply<'a>(
        input: PassBy<Self>,
        params: &P,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (a, b, c, d) = input;
        A::apply_into(a, params, dest);
        B::apply_into(b, params, dest);
        C::apply_into(c, params, dest);
        D::apply_into(d, params, dest)
    }
}

impl<A, B, C, D, E, P> Apply<P> for (A, B, C, D, E)
where
    A: SizePromotion<(B, C, D, E)>,
    A: ApplyInto<A::Output, P>,
    B: ApplyInto<A::Output, P>,
    C: ApplyInto<A::Output, P>,
    D: ApplyInto<A::Output, P>,
    E: ApplyInto<A::Output, P>,
{
    type Output = A::Output;

    fn apply<'a>(
        input: PassBy<Self>,
        params: &P,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (a, b, c, d, e) = input;
        A::apply_into(a, params, dest);
        B::apply_into(b, params, dest);
        C::apply_into(c, params, dest);
        D::apply_into(d, params, dest);
        E::apply_into(e, params, dest)
    }
}

impl<A, B, C, D, E, F, P> Apply<P> for (A, B, C, D, E, F)
where
    A: SizePromotion<(B, C, D, E, F)>,
    A: ApplyInto<A::Output, P>,
    B: ApplyInto<A::Output, P>,
    C: ApplyInto<A::Output, P>,
    D: ApplyInto<A::Output, P>,
    E: ApplyInto<A::Output, P>,
    F: ApplyInto<A::Output, P>,
{
    type Output = A::Output;

    fn apply<'a>(
        input: PassBy<Self>,
        params: &P,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (a, b, c, d, e, f) = input;
        A::apply_into(a, params, dest);
        B::apply_into(b, params, dest);
        C::apply_into(c, params, dest);
        D::apply_into(d, params, dest);
        E::apply_into(e, params, dest);
        F::apply_into(f, params, dest)
    }
}

impl<A, B, C, D, E, F, G, P> Apply<P> for (A, B, C, D, E, F, G)
where
    A: SizePromotion<(B, C, D, E, F, G)>,
    A: ApplyInto<A::Output, P>,
    B: ApplyInto<A::Output, P>,
    C: ApplyInto<A::Output, P>,
    D: ApplyInto<A::Output, P>,
    E: ApplyInto<A::Output, P>,
    F: ApplyInto<A::Output, P>,
    G: ApplyInto<A::Output, P>,
{
    type Output = A::Output;

    fn apply<'a>(
        input: PassBy<Self>,
        params: &P,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (a, b, c, d, e, f, g) = input;
        A::apply_into(a, params, dest);
        B::apply_into(b, params, dest);
        C::apply_into(c, params, dest);
        D::apply_into(d, params, dest);
        E::apply_into(e, params, dest);
        F::apply_into(f, params, dest);
        G::apply_into(g, params, dest)
    }
}

impl<A, B, C, D, E, F, G, H, P> Apply<P> for (A, B, C, D, E, F, G, H)
where
    A: SizePromotion<(B, C, D, E, F, G, H)>,
    A: ApplyInto<A::Output, P>,
    B: ApplyInto<A::Output, P>,
    C: ApplyInto<A::Output, P>,
    D: ApplyInto<A::Output, P>,
    E: ApplyInto<A::Output, P>,
    F: ApplyInto<A::Output, P>,
    G: ApplyInto<A::Output, P>,
    H: ApplyInto<A::Output, P>,
{
    type Output = A::Output;

    fn apply<'a>(
        input: PassBy<Self>,
        params: &P,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (a, b, c, d, e, f, g, h) = input;
        A::apply_into(a, params, dest);
        B::apply_into(b, params, dest);
        C::apply_into(c, params, dest);
        D::apply_into(d, params, dest);
        E::apply_into(e, params, dest);
        F::apply_into(f, params, dest);
        G::apply_into(g, params, dest);
        H::apply_into(h, params, dest)
    }
}
