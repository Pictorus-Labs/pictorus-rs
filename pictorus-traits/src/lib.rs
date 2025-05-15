//! This repository defines the core traits that define what a "block" is in the Pictorus platform. Additionally this repository contains
//! the implementations of the blocks provided with Pictorus.
//!
//! # Pictorus Trait Design
//! These traits define what a "block" is in Pictorus. Blocks are the fundamental unit of computation used in the GUI, we provide a library
//! of blocks that cover most use cases, however a major goal of this trait system is to allow users to implement custom functionality by
//! writing their own implementation of these traits. The Block traits provide a consistent interface that allows the Front end and Code
//! generator to work with all blocks (custom and otherwise) the same way.

#![no_std]
// and conditionally no_alloc

use core::mem;
use core::time::Duration;

mod sealed;
use sealed::Sealed;

pub mod custom_blocks;
pub use custom_blocks::*;

/// A processing block
pub trait ProcessBlock: Default {
    // NOTE because of the `Inputs` trait bound; all blocks must have at least *one* input
    type Inputs: Pass;
    type Output: Pass;
    type Parameters;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        context: &dyn Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output>;
}

pub trait HasIc: ProcessBlock {
    fn new(parameters: &Self::Parameters) -> Self;
}

/// A generator block
///
/// This block has no inputs
pub trait GeneratorBlock: Default {
    type Parameters;
    type Output: Pass;

    fn generate(
        &mut self,
        parameters: &Self::Parameters,
        context: &dyn Context,
    ) -> PassBy<Self::Output>;
}

/// An output block
///
/// This block has no output signals and usually performs a "side effect" instead of outputting data.
pub trait OutputBlock {
    type Inputs: Pass;
    type Parameters;

    fn output(
        &mut self,
        parameters: &Self::Parameters,
        context: &dyn Context,
        inputs: PassBy<'_, Self::Inputs>,
    );
}

/// An input block
///
/// This block has no inputs signals. Unlike a `GeneratorBlock` it outputs data from the real world rather
/// than synthetic data.
pub trait InputBlock {
    type Output: Pass;
    type Parameters;

    fn input(
        &mut self,
        parameters: &Self::Parameters,
        context: &dyn Context,
    ) -> PassBy<'_, Self::Output>;
}

/// The execution context
// this trait avoids leaking types associated to the "runtime" into the signature of
// `{Block,Generator}::run`
pub trait Context {
    // This is defined as the actual elapsed time since the last tick, Will return None if the
    // model is on its first tick
    fn timestep(&self) -> Option<Duration>;
    /// Time elapsed since the start of the program / simulation
    fn time(&self) -> Duration;
    // Fundamental Timestep, The goal timestep for the model
    fn fundamental_timestep(&self) -> Duration;
}

/// Data can be passed between blocks
pub trait Pass: Sealed + 'static {
    /// Whether the data is passed by value or by reference
    type By<'a>: Copy;

    fn as_by(&self) -> Self::By<'_>;
}

pub type PassBy<'a, T> = <T as Pass>::By<'a>;

impl<T> Pass for T
where
    T: Scalar,
{
    type By<'a> = Self;

    fn as_by(&self) -> Self::By<'_> {
        *self
    }
}

/// "Scalar" types
///
/// Marker trait for small primitives like floats, integers and booleans
pub trait Scalar: Sealed + Copy + 'static + Default + Into<f64> {}

impl Scalar for bool {}
impl Sealed for bool {}

impl Scalar for u8 {}
impl Sealed for u8 {}

impl Scalar for i8 {}
impl Sealed for i8 {}

impl Scalar for u16 {}
impl Sealed for u16 {}

impl Scalar for i16 {}
impl Sealed for i16 {}

impl Scalar for u32 {}
impl Sealed for u32 {}

impl Scalar for i32 {}
impl Sealed for i32 {}

impl Scalar for f32 {}
impl Sealed for f32 {}

impl Scalar for f64 {}
impl Sealed for f64 {}

/// Auto-promotion
pub trait Promote<RHS: Scalar>: Scalar {
    type Output: Scalar
        + core::ops::Add<Output = Self::Output>
        + core::ops::Mul<Output = Self::Output>
        + core::ops::Sub<Output = Self::Output>
        + core::ops::Div<Output = Self::Output>;

    fn promote_left(self) -> Self::Output;
    fn promote_right(rhs: RHS) -> Self::Output;
}

macro_rules! promotions {
    ($( ( $($from:ident),* ) -> $to:ident ),*) => {
        $(
            impl Promote<$to> for $to {
                type Output = $to;

                fn promote_left(self) -> Self::Output {
                    self
                }

                fn promote_right(rhs: $to) -> Self::Output {
                    rhs
                }
            }

            $(
                impl Promote<$from> for $to {
                    type Output = $to;

                    fn promote_left(self) -> Self::Output {
                        self
                    }

                    fn promote_right(rhs: $from) -> Self::Output {
                        rhs as $to
                    }
                }

                impl Promote<$to> for $from {
                    type Output = $to;

                    fn promote_left(self) -> Self::Output {
                        self as $to
                    }

                    fn promote_right(rhs: $to) -> Self::Output {
                        rhs
                    }
                }
            )*
        )*
    };
}

// TODO add more impls are needed
promotions! {
    (u8, u16) -> f32,
    (f32) -> f64
}

pub type Promotion<L, R> = <L as Promote<R>>::Output;

// a fixed-size array is like a mathematical vector
// NOTE the `Pod` trait bound prevents the creation of nested vectors
impl<const N: usize, T> Pass for [T; N]
where
    T: Scalar,
{
    type By<'o> = &'o Self;

    fn as_by(&self) -> Self::By<'_> {
        self
    }
}

impl<const N: usize, T> Sealed for [T; N] where T: Scalar {}

/// This is a Zero-Size-Type that is used as a stand-in for `[u8]` when using the `Pass` trait
/// Because `[u8]` is a dynamically-sized type it is not possible to use something like `([u8], [u8])` as a generic
/// parameter. This type is used to work around that limitation. It defines `By = &[u8]` so the correct type is still
/// passed to or from blocks
pub struct ByteSliceSignal;

impl Sealed for ByteSliceSignal {}

impl Pass for ByteSliceSignal {
    type By<'a> = &'a [u8];

    fn as_by(&self) -> Self::By<'_> {
        &[]
    }
}

/// Matrix in column-major order
// this type is only used as a "DTO" (Data Transfer Object). meaning that this type has
// no methods and it does NOT enforce any sort of invariants (which is why its field is public).
// it's only used to *transfer* data between blocks
//
// utility functions to convert between this and third-party crates like `nalgebra` are to be
// provided in a different crate. this also keeps this "interface" crate free from
// third-party dependencies
// XXX should this encode row-order vs column-order?
// NOTE the `Scalar` trait bound prevents the creation of nested matrices
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Matrix<const NROWS: usize, const NCOLS: usize, T>
where
    T: Scalar,
{
    pub data: [[T; NROWS]; NCOLS],
}

impl<const NROWS: usize, const NCOLS: usize, T> Default for Matrix<NROWS, NCOLS, T>
where
    T: Scalar,
{
    fn default() -> Self {
        Self::zeroed()
    }
}

impl<const NROWS: usize, const NCOLS: usize, T> Matrix<NROWS, NCOLS, T>
where
    T: Scalar,
{
    pub fn zeroed() -> Self {
        // SAFETY: `T: Scalar` is "sealed" so we know all the types it could be instantiated to and
        // we also know they are all primitives / "plain old data" so all bits set to zero is
        // a valid representation
        Self {
            data: unsafe { mem::zeroed() },
        }
    }
}

impl<const NROWS: usize, const NCOLS: usize, T> Pass for Matrix<NROWS, NCOLS, T>
where
    T: Scalar,
{
    type By<'a> = &'a Self;

    fn as_by(&self) -> Self::By<'_> {
        self
    }
}

impl<const NROWS: usize, const NCOLS: usize, T> Sealed for Matrix<NROWS, NCOLS, T> where T: Scalar {}

impl Pass for () {
    type By<'a> = Self;

    fn as_by(&self) -> Self::By<'_> {
        *self
    }
}

impl Sealed for () {}

impl<A, B> Pass for (A, B)
where
    A: Pass,
    B: Pass,
{
    type By<'a> = (PassBy<'a, A>, PassBy<'a, B>);

    fn as_by(&self) -> Self::By<'_> {
        (self.0.as_by(), self.1.as_by())
    }
}

impl<A, B> Sealed for (A, B)
where
    A: Pass,
    B: Pass,
{
}

impl<A, B, C> Pass for (A, B, C)
where
    A: Pass,
    B: Pass,
    C: Pass,
{
    type By<'a> = (PassBy<'a, A>, PassBy<'a, B>, PassBy<'a, C>);

    fn as_by(&self) -> Self::By<'_> {
        (self.0.as_by(), self.1.as_by(), self.2.as_by())
    }
}
impl<A, B, C> Sealed for (A, B, C)
where
    A: Pass,
    B: Pass,
    C: Pass,
{
}

impl<A, B, C, D> Pass for (A, B, C, D)
where
    A: Pass,
    B: Pass,
    C: Pass,
    D: Pass,
{
    type By<'a> = (PassBy<'a, A>, PassBy<'a, B>, PassBy<'a, C>, PassBy<'a, D>);

    fn as_by(&self) -> Self::By<'_> {
        (
            self.0.as_by(),
            self.1.as_by(),
            self.2.as_by(),
            self.3.as_by(),
        )
    }
}
impl<A, B, C, D> Sealed for (A, B, C, D)
where
    A: Pass,
    B: Pass,
    C: Pass,
    D: Pass,
{
}

impl<A, B, C, D, E> Pass for (A, B, C, D, E)
where
    A: Pass,
    B: Pass,
    C: Pass,
    D: Pass,
    E: Pass,
{
    type By<'a> = (
        PassBy<'a, A>,
        PassBy<'a, B>,
        PassBy<'a, C>,
        PassBy<'a, D>,
        PassBy<'a, E>,
    );

    fn as_by(&self) -> Self::By<'_> {
        (
            self.0.as_by(),
            self.1.as_by(),
            self.2.as_by(),
            self.3.as_by(),
            self.4.as_by(),
        )
    }
}
impl<A, B, C, D, E> Sealed for (A, B, C, D, E)
where
    A: Pass,
    B: Pass,
    C: Pass,
    D: Pass,
    E: Pass,
{
}

impl<A, B, C, D, E, F> Pass for (A, B, C, D, E, F)
where
    A: Pass,
    B: Pass,
    C: Pass,
    D: Pass,
    E: Pass,
    F: Pass,
{
    type By<'a> = (
        PassBy<'a, A>,
        PassBy<'a, B>,
        PassBy<'a, C>,
        PassBy<'a, D>,
        PassBy<'a, E>,
        PassBy<'a, F>,
    );

    fn as_by(&self) -> Self::By<'_> {
        (
            self.0.as_by(),
            self.1.as_by(),
            self.2.as_by(),
            self.3.as_by(),
            self.4.as_by(),
            self.5.as_by(),
        )
    }
}
impl<A, B, C, D, E, F> Sealed for (A, B, C, D, E, F)
where
    A: Pass,
    B: Pass,
    C: Pass,
    D: Pass,
    E: Pass,
    F: Pass,
{
}

impl<A, B, C, D, E, F, G> Pass for (A, B, C, D, E, F, G)
where
    A: Pass,
    B: Pass,
    C: Pass,
    D: Pass,
    E: Pass,
    F: Pass,
    G: Pass,
{
    type By<'a> = (
        PassBy<'a, A>,
        PassBy<'a, B>,
        PassBy<'a, C>,
        PassBy<'a, D>,
        PassBy<'a, E>,
        PassBy<'a, F>,
        PassBy<'a, G>,
    );

    fn as_by(&self) -> Self::By<'_> {
        (
            self.0.as_by(),
            self.1.as_by(),
            self.2.as_by(),
            self.3.as_by(),
            self.4.as_by(),
            self.5.as_by(),
            self.6.as_by(),
        )
    }
}
impl<A, B, C, D, E, F, G> Sealed for (A, B, C, D, E, F, G)
where
    A: Pass,
    B: Pass,
    C: Pass,
    D: Pass,
    E: Pass,
    F: Pass,
    G: Pass,
{
}

impl<A, B, C, D, E, F, G, H> Pass for (A, B, C, D, E, F, G, H)
where
    A: Pass,
    B: Pass,
    C: Pass,
    D: Pass,
    E: Pass,
    F: Pass,
    G: Pass,
    H: Pass,
{
    type By<'a> = (
        PassBy<'a, A>,
        PassBy<'a, B>,
        PassBy<'a, C>,
        PassBy<'a, D>,
        PassBy<'a, E>,
        PassBy<'a, F>,
        PassBy<'a, G>,
        PassBy<'a, H>,
    );

    fn as_by(&self) -> Self::By<'_> {
        (
            self.0.as_by(),
            self.1.as_by(),
            self.2.as_by(),
            self.3.as_by(),
            self.4.as_by(),
            self.5.as_by(),
            self.6.as_by(),
            self.7.as_by(),
        )
    }
}
impl<A, B, C, D, E, F, G, H> Sealed for (A, B, C, D, E, F, G, H)
where
    A: Pass,
    B: Pass,
    C: Pass,
    D: Pass,
    E: Pass,
    F: Pass,
    G: Pass,
    H: Pass,
{
}
