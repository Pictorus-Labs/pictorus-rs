# Notes and Tips for Implementing Blocks

The process block trait is defined like this:

```rust
pub trait ProcessBlock: Default {
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
```

Note that the `Input` and `Output` associated types must implement `Pass`. This means that when concretized by the compiler they will generally looks like the following:

- `f64` - A single scalar value
- `Matrix<5, 3, f32>` - a 5x3 matrix of `f32` values
- `(f64, Matrix<1,3, f63>)` - A tuple of a single scalar and a 1x3 matrix. This is used to represent multiple input edges or output edges

## Example

If you were to think about how you would design a simple `Bias` block, a good first step would be to define a Generic `I: Pass` that is defined as the input type (i.e. a single scalar, a matrix). We assume for now that the input and bias parameter must either be `f32` or `f64` but not a combination of both.

```rust
pub struct Parameters<P: Scalar> {
    bias: P //This could be say f64 or f32
}

// We need a separate P and I because the Input I can be a matrix or array, while the Parameter value P must always be just a Scalar.
pub struct BiasBlock<P: Scalar, I: Pass> {
    todo!()
}

impl<P: Scalar, I: Scalar> ProcessBlock for BiasBlock<P, I> {
    type Input: I; // The input type defines the output type
    type Output: I;
    type Parameters: Parameters<P>

    ...
}
```

Some example concretizations of this would be:

- `BiasBloc<f32, f32>`
- `BiasBlock<f64, f64>`
- `BiasBlock<f64, Matrix<4,3,f64>>`

But notably the following would also be valid in the way we have defined our struct and its generics, but are non-sensical to the way we have defined our problem

- `BiasBlock<f32, f64>` - Mixed types?
- `BiasBlock<f64, (f32, Matrix<4,3,f32>)>` - Trying to provide a multi-edge input

### Handwriting impls

The naive solution here would be to not make our implementation generic (or at least as generic), creating a separate `impl` fo each valid combination:

```rust
impl ProcessBlock for BiasBlock<f32, f32> {...}
impl ProcessBlock for BiasBlock<f64, f64> {...}
impl<const NROWS: usize, const NCOLS: usize> for BiasBlock<f64, Matrix<NROWS, NCOLS, f64>> {...}
impl<const NROWS: usize, const NCOLS: usize> for BiasBlock<f32, Matrix<NROWS, NCOLS, f32>> {...}
```

There is nothing wrong with this solution, however it is quite verbose. For this smaller block that can only accept a single edge as input that verbosity isn't too bad, but it is easy to imagine why this would break down quickly for blocks that a varying number of edges.

Additionally, it is pretty clear looking at these impls that they are over-constrained. If we are manually specifying each impl the block doesn't need to specify the parameter type separately because the developer can do that:

```rust
pub struct Parameters<T: Scalar> {
    bias: T //This could be say f64 or f32
}

pub struct BiasBlock<I: Pass> {
    buffer: I // The storage for the output data, only works because the output of `Bias` is the same shape as its input
}

impl ProcessBlock for BiasBlock<f32> {
    type Input: f32;
    type Output: f32;
    type Parameters: Parameters<f32>
}
impl<const NROWS: usize, const NCOLS: usize> for BiasBlock< Matrix<NROWS, NCOLS, f64>> {
    type Input:  Matrix<NROWS, NCOLS, f64>;
    type Output:  Matrix<NROWS, NCOLS, f64>;
    type Parameters: Parameters<f64>
}
```

Notably, this approach precludes being able to have mixed types for the parameter and input (e.g. `f32` and `f64`).

### Making Generics Work

If we decide that having to hand write all possible impls pf `ProcessBlock` is not viable we need to think about how to write a blanket impl. The key insight here will be to write a brand new trait for this specific block. By convention this has been called `Apply`. We can define this trait as below:

```rust
pub trait Apply<G: Scalar>: Pass { //G is the type of the gain parameter
    type Output: Pass;

    fn apply<'s>(
        store: &'s mut Option<Self::Output>,
        input: PassBy<Self>,
        gain: G,
    ) -> PassBy<'s, Self::Output>;
}
```

This trait is constrained so that it can only be implemented for types that already implement `Pass`. The idea is that we will implement this for possible input types, allowing them to specify their own output type, it allows us to write one impl of `ProcessBlock` that will cover all valid situations:

```rust
impl<G, T> ProcessBlock for BiasBlock<G, T>
where
    G: Scalar,
    T: Apply<G>,
{
    type Inputs = T;
    type Output = T::Output;
    type Parameters = Parameters<G>;

    fn process(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        input: PassBy<Self::Inputs>,
    ) -> PassBy<Self::Output> {
        T::apply(&mut self.buffer, input, parameters.gain);
    }
}
```

Now we can implement `Apply` for any type we want to accept as input:

```rust
/// Notice that we can cover both the float cases in one shot now
impl<T: Scalar + Float> Apply<T> for T {
    type Output: Self;

    ...
}

impl<T: Scalar + Float, const NROWS: usize, const NCOLS: usize> Apply<T> for Matrix<NROWS, NCOLS, T> {
    type Output = Self;

    ...
}

```

Here we note that the trait has a generic unlike the `ProcessBlock` trait. This has the added benefit of allowing to implement multiple "versions" of `Apply` for the same type (i.e. we could implement `Apply<f32>` and `Apply<f64>` for `f64`) if we want to support mixed types down the road.

### Conclusion notes

When we are implementing `ProcessBlock` for a block we need to be able to indicate to the compiler the types of the Inputs, Outputs, and Parameter. Without the `Apply` this can become impossible to do in a generic way if the types are not either all specified as separate generics over the block (which ends up being way over specified) or don't happen to always be the same (e.g. in the Bias block above the type of Input and Output will always be the same but that certainly isn't always the case). The solution is to either handwrite many concrete impls, or introduce `Apply` as layer of indirection. This allows us to do things like the following:

```rust
impl ProcessBlock for FooBlock<I: Apply> {
    type Input: I;
    type Output: <I as Apply>::Output;
    ...
}


impl<T: Scalar + Float, const NROWS: usize, const NCOLS: usize> Apply<T> for Matrix<NROWS, NCOLS, T> {
    type Output: T;
}
```

In this case we were able to "extract" a generic out of `I` which from the perspective of the `ProcessBlock` impl is one single type (i.e. there is no way to say "If `I` is a `Matrix` grab its third generic" from within the `ProcessBlock` impl). This extra layer of indirection adds complexity when writing blocks, but lets us encode rules about block input and output shapes and types in a more generic way that hand writing an impl for every situation

## Too complex????

Obviously this would be a lot for most users to deal with just to write their own block, however we are only contending with this complexity because we are writing the most general version of the most foundational blocks. A more representative example for a user's custom block might be something like this:

```rust

/// Takes a bunch of sensor in and then outputs a location in X,Y,Z
pub struct SensorFuser {
    buffer: Matrix<1,3,f32>
}

pub struct FuserParams {
    gain: f32,
    filter_cutoff: f32,
}

impl ProcessBlock for SensorFuser {
    type Input: (f64, Matrix<1,3,f32>, Matrix<3,3,f32>) // Say a altimeter, unfused X,Y,Z estimate from some other sensor, and separate Accel,Gyro,Mag combo sensor
    type Output: Matrix<1,3,f32> // X,Y,Z
    type Parameters: FusedParams,

    fn process(...) -> ... {
        //Do all sort of fun PhD math in here
    }
}

```

## Additional things explain in the future

Promotion trait and how it intersects with the above.
