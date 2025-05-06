# Pictorus Core Library

This crate defines the core traits that define what a "block" is in the Pictorus platform.

# Pictorus Trait Design

Blocks are the fundamental unit of computation used in the Pictorus GUI. We provide a library of blocks that cover most use cases, however a major goal of this trait system is to allow users to implement custom functionality by writing their own implementation of these traits. The Block traits provide a consistent interface that allows the Front End and Code Generator to work with all blocks (custom and otherwise) the same way.

## Trait Design Goals

1. Work in `std` and `no_std` environments

   - Don't require heap allocation
   - Make efficient use of memory and compute resources

2. Be general enough to cover all block functionality we support
3. Effectively model the "atomic time-step" model of computation that Pictorus targets
4. Be amenable to use with our UI and code generation
5. Be as easy to develop against as possible given the above

## Blocks

Blocks can be one of four types:

- Generators: Take no inputs and provide output
- Process: Take input and provide output
- Input: Take no input in the system graph and provide output, their output is expected to be a result of their interaction with external interfaces (e.g. UDP stream, SPI Device, UART stream, etc.)
- Output: Takes input and doesn't provide an output to the model graph. Similar to Input blocks, these provide no output to the system graph but are expected to have some sort of side-effect or external output as telemetry, hardware interaction, etc.

Any block that has an output edge (i.e. all but `Output` blocks) must store a copy of its output as a member of the block itself (or at the very least somewhere with the same lifetime as the block itself). This is because output from a block is distributed as an immutable reference to that internal copy which is then passed to any block(s) that use that output as input into themselves. ( This is a slight fib, `Scalar` values are returned by value and therefore a block does not technically need to keep a copy of the output value locally)

Every block trait defines a "tick" function (`generate`, `process`, `input`, and `output` respectively for the list above). In a Pictorus application a given model is executed by running the tick function of every block once per time step. Blocks should never assume they will be run at a specific time-step since that is globally configurable. Further, if at all possible, a block should not assume that the application will perfectly maintain consistent time-steps during execution. Blocks are passed a [`Context`] during each tick that gives the true timing of a tick which should be used to ensure that delayed or skipped time-steps don't cause anymore disruption to the output than necessary (e.g. If you are generating a sine wave and time-step is delayed the output should give the value of that sine function at the new tick-time not what it would have been if we hadn't been delayed, this avoids drift of a long running system). Although execution of every block in a model takes a non-zero amount of time, during a given time-step every block that is called will be passed the same time in the context they are given so that all the computation in a given time-step is "atomic" with respect to all the blocks that use timing information in their execution.

### The Block Traits

#### `GeneratorBlock`

Defines an `Output` associated type that must impl `Pass`.

Defines a "tick" function of `generate` that must be able to generate an output using some combination of the passed in `Context`, the passed in `Parameters` and the block's internal state

#### `ProcessBlock`

These make up of the majority of blocks in Pictorus. They must define an `Output` associated type with the same restrictions as described above. They additionally define an `Input` associated type with the same bounds.

The tick function `process` is passed in the blocks parameters, a `Context` and an Input and returns an Output. Blocks may also have internal state that affects the output for a given set of arguments passed into `process`

#### `InputBlock`

Defines an `Output` associated type that must impl `Pass`.

It is expected that on each call of the "tick" function the output generated (if any) will be from some sort of external source (e.g. sensor hardware, a web socket, etc.). For this reason an Input Block will almost always have to be tightly coupled to the rest of the application that the model is being compiled into

#### `OutputBlock`

Defines an `Input` associated type that must impl `Pass`.

As with the Input Block, implementors of this trait are expected to be Outputting data to some device or interface external to the model, and will also be tightly coupled to the rest of the application.

## Edges

Edges in the system graph represent data traveling between blocks. Data is transmitted once per system tick. The data can take the following forms:

- Scalar: A single value, through generics it can be an `f32`, `f64`, `bool`, or `u8`
- A fixed size 2D matrix: All the elements of a given matrix must be the same type, that type can be any of the scalar types
- A `ByteSliceSignal` which represents a 1D slice of `u8`: Used for communicating byte streams

## Parameters

Parameters alter the behavior and sometimes connectivity of blocks. There exist two types of parameters; Compile Time Parameters and Run Time Parameters, both types of parameters must be set to a value in the UI during model development.

Compile Time Parameters are parameters that affect the size, type, or number of input and output edges of a block or that affect the generated code for interfacing with hardware (e.g. PWM pin selection), they are relatively easy to identify as they will appear as generics in the block definition. They cannot be changed once a model has been compiled into an executable.

Runtime Parameters on the other hand will use the value set during model development by default, but can later be changed at runtime, or when the model is launched. Each block implementation indicates its Runtime Parameters by setting the `Parameters` associated type of their trait. Each call to a given block's "tick" function will always be passed an immutable reference to one of these `Parameters`. This design allows the application that the model has been compiled into to handle the details of parameter management as an implementation detail beyond the scope of responsibility of Blocks.

Alternatives were explored to have Blocks store their own parameters with getter and setter functions, or to have each block be given a mutex guarded pointer to a copy of their parameters at construction. However, the design described above was chosen because it offered the best ergonomics for block writers while still allowing parameters to be changed, saved, etc.

## Context and Runtime

The `Runtime` is not a formal trait but should be a platform specific way to track and control the flow of time. The `Runtime` is responsible for keeping track of the elapsed time, the timestep increment, and correctly incrementing time. Incrementing the timestep is platform and framework specific and may be a software timer, hardware timer, async method, or another approach to ensure the program time "ticks" in a controlled way.

A `Runtime` should generate a struct implementing the `Context` trait at the start of a "tick" which is an immutable representation of elapsed time and timestep increment of the program for the current "tick" iteration. A `Context` is required to be passed into any `Block` that is runnable, which may or may not use the `Context` internally to process data.

## Edge Details

In the Pictorus application, edges are the connections between blocks. As far as blocks are concerned they must define their `Inputs` and `Outputs` types as appropriate for their functionality and the Pictorus code generation will handle the work of actually assigning block outputs to variables and passing them into to blocks as parameters of their "tick" function.

In the case a block has more than one output it is still modeled as a single `Inputs` type. That single type will be a tuple of each of the individual input edges the block needs.

In order to support multiple outputs from a block (e.g. "Is Stale" on quite a few of our existing blocks), the `Outputs` associated type should be defined as a tuple of the outputs. Codegen can then use [tuple destructuring](https://doc.rust-lang.org/rust-by-example/flow_control/match/destructuring/destructure_tuple.html) when assigning names to the outputs (e.g. `let (foo_data, foo_is_stale) = foo_block.process(...);` ).

### Edge Data Types

The `Pass` trait bounds are the top level trait bounds on the various input and output types of the block traits. Those traits are both "Sealed" and such block implementors can not implement them over arbitrary types. The details of those two traits are explained in the following section but this section will explain the set of types that implement them.

The following types can be used as edge data:

- `Scalar`s: The `Scalar` trait is implemented for; `f32`, `f64`, `u8`, `u16`, and `bool`. They represent the individual values in the system
- [`ByteSliceSignal`]: Used for byte streams, this is essentially a stand-in for `[u8]`. This is necessary because `[u8]` is not a `Sized` type and therefore there are constraints on where it can show up in types definitions.
- `Matrix<const NROWS: usize, const NCOLS: usize, T: Scalar>`: This type is used to model matrices of a singular scalar type (as well as vectors; being a special case of matrices where one dimension has a size of 1). They have a fixed size that must be known at compile time and don't require on `alloc`. Every element of a given matrix must be the same scalar type

And finally to support blocks with multiple inputs tuples composed of all types that implement `Pass` will also implement `Pass`. That is:

```rust
impl<A: Pass> Pass for (A){...}
impl<A: Pass, B: Pass> Pass for (A, B){...}
impl<A: Pass, B: Pass, C: Pass> for (A, B, C) {...}
etc...
```

### Pass Trait

The Pass trait allows Scalar types to be passed by value (i.e. by being copied) when being passed between blocks, while all other types are passed by reference. The trait is defined as:

```rust
/// Data can be passed between blocks
pub trait Pass: Sealed + 'static {
    /// Whether the data is passed by value or by reference
    type By<'a>: Copy;
}
```

And for convenience we define a type alias `PassBy`:

```rust
pub type PassBy<'a, T> = <T as Pass>::By<'a>;
```

Together these allow us to indicate for any type where that type should be passed by value or by reference when used as an edge. The following blanket impl indicates that all `Scalar` types will be passed by value

```rust
impl<T> Pass for T
where
    T: Scalar,
{
    type By<'a> = Self;

    fn as_by(&self) -> Self::By<'_> {
        *self
    }
}
```

And here is an example implementation for `ByteSliceSignal` slices:

```rust
impl Pass for ByteSliceSignal {
    type By<'a> = &'a [u8];

    fn as_by(&self) -> Self::By<'_> {
        &[]
    }
}
```

Here you can see that `PassBy<'a, ByteSliceSignal> = &'a [u8]`. Similar implementations are provided for the other collection edge data types.

Having `PassBy<'_, T>` available for any type that can be used as edge data allows our block traits to use a definition like this example from the `ProcessBlock`:

```rust
fn process<'b>(
    &'b mut self,
    context: &dyn Context,
    inputs: PassBy<'_, Self::Inputs>,
) -> PassBy<'b, Self::Output>;
```

Here you can see that for a block that accepted `DMatrix<f64>` and returned a `ByteSliceSignal` it would desugar into the following:

```rust
fn process<'b>(
    &'b mut self,
    context: &dyn Context,
    inputs: &DMatrix<f64>
) -> u8;
```

and in that way the scalar will be returned by value and the DMatrix will be accepted by reference.

Finally, this approach scales up naturally when we are using a tuple to accept multiple inputs. As an example look at this 3 member tuple implementation:

```rust
impl<A, B, C> Pass for (A, B, C)
where
    A: Pass,
    B: Pass,
    C: Pass,
{
    type By<'a> = (PassBy<'a, A>, PassBy<'a, B>, PassBy<'a, C>);
}
```

It can be seen than even when some types want to be passed by value and others by reference the `PassBy<'a, (A,B,C)> = (PassBy<'a, A>, PassBy<'a, B>, PassBy<'a, C> )`

### Promotion

The Rust core library is extremely deliberate about conversions between primitive numeric types only offering infallible conversions where there can be no overflow, or other loss of information (e.g. u8 -> u16 is infallible but i32 -> u64 is fallible since u64 cannot represent negative values). In order to handle similar cases in our core library we offer the `Promotion` trait:

```rust
pub trait Promote<RHS: Scalar>: Scalar {
    type Output: Scalar;

    fn promote_left(self) -> Self::Output;
    fn promote_right(rhs: RHS) -> Self::Output;
}
```

When a scalar primitive type impls `Promote<T>` for some other scalar type the `Output` associated type indicates the type that can hold data from both types without loss of information. For convenience we offer the `Promotion<L,R>` type, defined as `pub type Promotion<L, R> = <L as Promote<R>>::Output;`. See the below example for how this plays out for the `u8`<->`f32` mapping.

```rust
<u8 as Promotion<f32>>::Output = f32
<f32 as Promotion<u8>>::Output = f32

// This implies:
// Promotion<u8, f32> == Promotion<f32, u8> == f32
```

It is worth noting that we implement no-op case of `impl<T:Scalar>  Promote<T> for T` for all of our base scalar types.

The end result is that Block writers can then describe their inputs and outputs using the Promotion trait and type. See this example of the GainBlock which allows the Gain parameter and the passed in data to be of potentially two different types while still allowing us to specify that the output of that block will be the type that allows us to apply that gain without loss of information.

```rust
impl<const N: usize, G, T> Apply<G> for [T; N]
where
    T: Scalar,
    G: Promote<T>,
    Promotion<G, T>: MulAssign,
{
    type Output = [Promotion<G, T>; N];

    fn apply<'s>(
        store: &'s mut Option<Self::Output>,
        input: PassBy<'_, Self>,
        gain: G,
    ) -> PassBy<'s, Self::Output> {
        let output = store.insert(input.map(<G as Promote<T>>::promote_right));
        let gain = Promote::promote_left(gain);
        output.iter_mut().for_each(|lhs| lhs.mul_assign(gain));
        output
    }
}
```

While this functionality is undoubtedly useful and we will want to move towards eventually offering it on all of the core blocks we offer that could benefit from it it doesn't have to be implemented for all of our blocks at first and certainly won't have to be used in custom blocks where a user knows the type of data they expect to receive or send. It is just a way to make blocks more general
