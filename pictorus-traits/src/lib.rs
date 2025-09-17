//! This crate defines the core traits that define what a "block" is in the Pictorus platform.
//!
//! # Pictorus Trait Design
//! Blocks are the fundamental unit of computation used in the Pictorus GUI. We provide a library
//! of blocks that cover most use cases, however a major goal of this trait system is to allow
//! users to implement custom functionality by writing their own implementation of these traits.
//! The Block traits provide a consistent interface that allows the Front End and Code Generator
//! to work with all blocks (custom and otherwise) the same way.
//!
//! ## Trait Design Goals
//! 1. Work in `std` and `no_std` environments
//!    - Don't require heap allocation
//!    - Make efficient use of memory and compute resources
//! 2. Be general enough to cover all block functionality we support
//! 3. Effectively model the "atomic time-step" model of computation that Pictorus targets
//! 4. Be amenable to use with our UI and code generation
//! 5. Be as easy to develop against as possible given the above
//!
//! ## Blocks
//! Blocks can be one of four types:
//! - Generators: Take no inputs and provide output
//! - Process: Take input and provide output
//! - Input: Take no input in the system graph and provide output, their output is expected
//!   to be a result of their interaction with external interfaces (e.g. UDP stream, SPI Device, UART stream, etc.)
//! - Output: Takes input and doesn't provide an output to the model graph. Similar to Input blocks,
//!   these provide no output to the system graph but are expected to have some sort of side-effect or external output as telemetry,
//!   hardware interaction, etc.
//!
//! Any block that has an output edge (i.e. all but `Output` blocks) must store a copy of its output as a member of the block itself
//! (or at the very least somewhere with the same lifetime as the block itself). This is because output from a block is distributed
//! as an immutable reference to that internal copy which is then passed to any block(s) that use that output as input into themselves.
//! (This is a slight fib, `Scalar` values are returned by value and therefore a block does not technically need to keep a copy of the
//! output value locally)
//!
//! Every block trait defines a "tick" function (`generate`, `process`, `input`, and `output` respectively for the list above).
//! In a Pictorus application a given model is executed by running the tick function of every block once per time step.
//! Blocks should never assume they will be run at a specific time-step since that is globally configurable.
//! Further, if at all possible, a block should not assume that the application will perfectly maintain consistent
//! time-steps during execution. Blocks are passed a [`Context`] during each tick that gives the true timing of a tick which
//! should be used to ensure that delayed or skipped time-steps don't cause anymore disruption to the output than necessary
//! (e.g. If you are generating a sine wave and time-step is delayed the output should give the value of that sine function
//! at the new tick-time not what it would have been if we hadn't been delayed, this avoids drift of a long running system).
//! Although execution of every block in a model takes a non-zero amount of time, during a given time-step every block that
//! is called will be passed the same time in the context they are given so that all the computation in a given time-step is
//! "atomic" with respect to all the blocks that use timing information in their execution.
//!
//! ### The Block Traits
//! #### `GeneratorBlock`
//! Defines an `Output` associated type that must impl `Pass`.
//!
//! Defines a "tick" function of `generate` that must be able to generate an output using some combination of the passed in `Context`,
//! the passed in `Parameters` and the block's internal state
//!
//! #### `ProcessBlock`
//! These make up of the majority of blocks in Pictorus. They must define an `Output` associated type with the same restrictions as
//! described above. They additionally define an `Input` associated type with the same bounds.
//!
//! The tick function `process` is passed in the blocks parameters, a `Context` and an Input and returns an Output.
//! Blocks may also have internal state that affects the output for a given set of arguments passed into `process`
//!
//! #### `InputBlock`
//! Defines an `Output` associated type that must impl `Pass`.
//!
//! It is expected that on each call of the "tick" function the output generated (if any) will be from some sort of external source
//! (e.g. sensor hardware, a web socket, etc.). For this reason an Input Block will almost always have to be tightly coupled to the
//! rest of the application that the model is being compiled into
//!
//! #### `OutputBlock`
//! Defines an `Input` associated type that must impl `Pass`.
//!
//! As with the Input Block, implementors of this trait are expected to be Outputting data to some device or interface external
//! to the model, and will also be tightly coupled to the rest of the application.
//!
//! ## Edges
//! Edges in the system graph represent data traveling between blocks. Data is transmitted once per system tick.
//! The data can take the following forms:
//! - Scalar: A single value, through generics it can be an `f32`, `f64`, `bool`, or `u8`
//! - A fixed size 2D matrix: All the elements of a given matrix must be the same type, that type can be any of the scalar types
//! - A `ByteSliceSignal` which represents a 1D slice of `u8`: Used for communicating byte streams
//!
//! ## Parameters
//! Parameters alter the behavior and sometimes connectivity of blocks. There exist two types of parameters;
//! Compile Time Parameters and Run Time Parameters, both types of parameters must be set to a value in the
//! UI during model development.
//!
//! Compile Time Parameters are parameters that affect the size, type, or number of input and output edges of a block or
//! that affect the generated code for interfacing with hardware (e.g. PWM pin selection), they are relatively easy to
//! identify as they will appear as generics in the block definition. They cannot be changed once a model has been compiled
//! into an executable.
//!
//! Runtime Parameters on the other hand will use the value set during model development by default, but can later be changed
//! at runtime, or when the model is launched. Each block implementation indicates its Runtime Parameters by setting the `Parameters`
//! associated type of their trait. Each call to a given block's "tick" function will always be passed an immutable reference to
//! one of these `Parameters`. This design allows the application that the model has been compiled into to handle the details of
//! parameter management as an implementation detail beyond the scope of responsibility of Blocks.
//!
//! Alternatives were explored to have Blocks store their own parameters with getter and setter functions,
//! or to have each block be given a mutex guarded pointer to a copy of their parameters at construction.
//! However, the design described above was chosen because it offered the best ergonomics for block writers while still allowing
//! parameters to be changed, saved, etc.
//!
//! ## Context and Runtime
//! The `Runtime` is not a formal trait but should be a platform specific way to track and control the flow of time.
//! The `Runtime` is responsible for keeping track of the elapsed time, the timestep increment, and correctly incrementing time.
//! Incrementing the timestep is platform and framework specific and may be a software timer, hardware timer, async method,
//! or another approach to ensure the program time "ticks" in a controlled way.
//!
//! A `Runtime` should generate a struct implementing the `Context` trait at the start of a "tick" which is an immutable
//! representation of elapsed time and timestep increment of the program for the current "tick" iteration.
//! A `Context` is required to be passed into any `Block` that is runnable, which may or may not use the `Context` internally
//! to process data.
//!
//! ## Edge Details
//! In the Pictorus application, edges are the connections between blocks. As far as blocks are concerned they must define their
//! `Inputs` and `Outputs` types as appropriate for their functionality and the Pictorus code generation will handle the work of
//! actually assigning block outputs to variables and passing them into to blocks as parameters of their "tick" function.
//!
//! In the case a block has more than one output it is still modeled as a single `Inputs` type. That single type will be a tuple
//! of each of the individual input edges the block needs.
//!
//! In order to support multiple outputs from a block (e.g. "Is Stale" on quite a few of our existing blocks), the `Outputs`
//! associated type should be defined as a tuple of the outputs.
//! Code generation can then use [tuple destructuring](https://doc.rust-lang.org/rust-by-example/flow_control/match/destructuring/destructure_tuple.html)
//! when assigning names to the outputs (e.g. `let (foo_data, foo_is_stale) = foo_block.process(...);` ).
//!
//! ### Edge Data Types
//! The `Pass` trait bounds are the top level trait bounds on the various input and output types of the block traits.
//! Those traits are both "Sealed" and such block implementors can not implement them over arbitrary types.
//! The details of those two traits are explained in the following section but this section will explain the set of types
//! that implement them.
//!
//! The following types can be used as edge data:
//! - `Scalar`s: The `Scalar` trait is implemented for; `f32`, `f64`, `u8`, `u16`, and `bool`.
//!   They represent the individual values in the system
//! - [`ByteSliceSignal`]: Used for byte streams, this is essentially a stand-in for `[u8]`.
//!   This is necessary because `[u8]` is not a `Sized` type and therefore there are constraints on where it can show up in types definitions.
//! - `Matrix<const NROWS: usize, const NCOLS: usize, T: Scalar>`: This type is used to model matrices of a singular scalar type
//!   (as well as vectors; being a special case of matrices where one dimension has a size of 1). They have a fixed size that must be known at compile time and don't require on `alloc`. Every element of a given matrix must be the same scalar type
//!
//! And finally to support blocks with multiple inputs tuples composed of all types that implement `Pass` will also implement `Pass`.
//! That is:
//!
//! ```rust ignore
//! impl<A: Pass> Pass for (A){...}
//! impl<A: Pass, B: Pass> Pass for (A, B){...}
//! impl<A: Pass, B: Pass, C: Pass> for (A, B, C) {...}
//! etc...
//! ```
//!
//! ### Pass Trait
//! The Pass trait allows Scalar types to be passed by value (i.e. by being copied) when being passed between blocks,
//! while all other types are passed by reference. The trait is defined as:
//!
//! ```rust ignore
//! /// Data can be passed between blocks
//! pub trait Pass: Sealed + 'static {
//!     /// Whether the data is passed by value or by reference
//!     type By<'a>: Copy;
//! }
//! ```
//!
//! And for convenience we define a type alias `PassBy`:
//!
//! ```rust ignore
//! pub type PassBy<'a, T> = <T as Pass>::By<'a>;
//! ```
//!
//! Together these allow us to indicate for any type where that type should be passed by value or by reference when used as an edge.
//! The following blanket impl indicates that all `Scalar` types will be passed by value
//!
//! ```rust ignore
//! impl<T> Pass for T
//! where
//!     T: Scalar,
//! {
//!     type By<'a> = Self;
//!
//!     fn as_by(&self) -> Self::By<'_> {
//!         *self
//!     }
//! }
//! ```
//!
//! And here is an example implementation for `ByteSliceSignal` slices:
//!
//! ```rust ignore
//! impl Pass for ByteSliceSignal {
//!     type By<'a> = &'a [u8];
//!
//!     fn as_by(&self) -> Self::By<'_> {
//!         &[]
//!     }
//! }
//! ```
//!
//! Here you can see that `PassBy<'a, ByteSliceSignal> = &'a [u8]`. Similar implementations are provided for the other collection
//! edge data types.
//!
//! Having `PassBy<'_, T>` available for any type that can be used as edge data allows our block traits to use a definition like
//! this example from the `ProcessBlock`:
//!
//! ```rust ignore
//! fn process<'b>(
//!     &'b mut self,
//!     context: &dyn Context,
//!     inputs: PassBy<'_, Self::Inputs>,
//! ) -> PassBy<'b, Self::Output>;
//! ```
//!
//! Here you can see that for a block that accepted `DMatrix<f64>` and returned a `ByteSliceSignal` it would desugar into the following:
//!
//! ```rust ignore
//! fn process<'b>(
//!     &'b mut self,
//!     context: &dyn Context,
//!     inputs: &DMatrix<f64>
//! ) -> u8;
//! ```
//!
//! and in that way the scalar will be returned by value and the DMatrix will be accepted by reference.
//!
//! Finally, this approach scales up naturally when we are using a tuple to accept multiple inputs.
//! As an example look at this 3 member tuple implementation:
//! ```rust ignore
//! impl<A, B, C> Pass for (A, B, C)
//! where
//!     A: Pass,
//!     B: Pass,
//!     C: Pass,
//! {
//!     type By<'a> = (PassBy<'a, A>, PassBy<'a, B>, PassBy<'a, C>);
//! }
//! ```
//!
//! It can be seen than even when some types want to be passed by value and others by reference the
//! `PassBy<'a, (A,B,C)> = (PassBy<'a, A>, PassBy<'a, B>, PassBy<'a, C> )`
//!
//! ### Promotion
//! The Rust core library is extremely deliberate about conversions between primitive numeric types only offering infallible
//! conversions where there can be no overflow, or other loss of information (e.g. u8 -> u16 is infallible but i32 -> u64 is
//! fallible since u64 cannot represent negative values). In order to handle similar cases in our core library we offer the
//! `Promotion` trait:
//!
//! ```rust ignore
//! pub trait Promote<RHS: Scalar>: Scalar {
//!     type Output: Scalar;
//!
//!     fn promote_left(self) -> Self::Output;
//!     fn promote_right(rhs: RHS) -> Self::Output;
//! }
//! ```
//!
//! When a scalar primitive type impls `Promote<T>` for some other scalar type the `Output` associated type indicates the
//! type that can hold data from both types without loss of information. For convenience we offer the `Promotion<L,R>` type,
//! defined as `pub type Promotion<L, R> = <L as Promote<R>>::Output;`.
//! See the below example for how this plays out for the `u8`<->`f32` mapping.
//!
//! ```rust ignore
//! <u8 as Promotion<f32>>::Output = f32
//! <f32 as Promotion<u8>>::Output = f32
//!
//! // This implies:
//! // Promotion<u8, f32> == Promotion<f32, u8> == f32
//! ```
//!
//! It is worth noting that we implement no-op case of `impl<T:Scalar>  Promote<T> for T` for all of our base scalar types.
//! The end result is that Block writers can then describe their inputs and outputs using the Promotion trait and type.
//! See this example of the GainBlock which allows the Gain parameter and the passed in data to be of potentially two different
//! types while still allowing us to specify that the output of that block will be the type that allows us to apply that gain
//! without loss of information.
//!
//! ```rust ignore
//! impl<const N: usize, G, T> Apply<G> for [T; N]
//! where
//!     T: Scalar,
//!     G: Promote<T>,
//!     Promotion<G, T>: MulAssign,
//! {
//!     type Output = [Promotion<G, T>; N];
//!
//!     fn apply<'s>(
//!         store: &'s mut Option<Self::Output>,
//!         input: PassBy<'_, Self>,
//!         gain: G,
//!     ) -> PassBy<'s, Self::Output> {
//!         let output = store.insert(input.map(<G as Promote<T>>::promote_right));
//!         let gain = Promote::promote_left(gain);
//!         output.iter_mut().for_each(|lhs| lhs.mul_assign(gain));
//!         output
//!     }
//! }
//! ```
//!
//! While this functionality is undoubtedly useful and we will want to move towards eventually offering it on all of the core
//! blocks we offer that could benefit from it it doesn't have to be implemented for all of our blocks at first and certainly
//! won't have to be used in custom blocks where a user knows the type of data they expect to receive or send.
//! It is just a way to make blocks more general.

#![no_std]
// and conditionally no_alloc

use core::mem;
use core::time::Duration;

mod sealed;
use sealed::Sealed;

pub mod custom_blocks;
pub use custom_blocks::*;

pub mod tuple_array_interop;

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
    ) -> PassBy<'_, Self::Output>;
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
pub trait Scalar: Sealed + Copy + 'static + Default + Into<f64> + PartialEq {}

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
// NOTE the `Scalar` trait bound prevents the creation of nested vectors
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

    pub fn is_truthy(&self) -> bool {
        let default = T::default();
        self.data.iter().flatten().any(|x| *x != default)
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
