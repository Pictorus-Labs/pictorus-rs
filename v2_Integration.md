## Status
Removed so far:
 - `InputBlock` and `OutputBlock` traits
 - `Promotion` trait
 - `BlockData` / `OldBlockData`
 - Blocks:
    - ADC
    - DAC
    - CAN TX/RX
    - GPIO In/Out
    - I2C In/Out
    - PassthroughBlock
    - PWM
    - Serial RX/TX
    - SPI RX/TX
    - UDP RX/TX
    - NoOp

Git Line Stats:
 - 521 Lines Added
 - 7466 Lines Removed

 - 96 files modified


## Context

This is already just a trait interface so we should be able to implement this context trait for the context struct in V2

I removed the `fundamental_timestep()` function from the trait since it wasn't actually used in any blocks and isn't modeled in our
V2 context. This leaves the following:
```rust
pub trait Context {
    // This is defined as the actual elapsed time since the last tick, Will return None if the
    // model is on its first tick
    fn timestep(&self) -> Option<Duration>;
    /// Time elapsed since the start of the program / simulation
    fn time(&self) -> Duration;
}
```

And this looks like it would be esssentially trivial to implement for our v2 `TimeContext` (or `ModelContext` which holds a `TimeContext`):
```rust
pub struct TimeContext {
    /// The total system time since start.
    system_time: Duration,
    /// The delta time that a model should advance itself by during an update cycle.
    delta_time: Duration,
}
```

Of course the V1 trait refers to the `use core::time::Duration` type while in `TimeContext` we are using a `Duration` type we defined ourselves. But translating between 
them should be no problem.

## Path Forward

### Generator Block

This could be moved to be a ProcessBlock where `type Input = ()` without difficulty I believe. The MAster plan dock does mention unifying into a single `Block` trait


### Removing Pass/PassBy

Part of the original plan, thought of as complication to allow return by value vs. return by reference optimization

However when I started on the work it became clear that the real heavy lifting that `Pass` and `PassBy` were doing was turning:
`(A, B, C)` into `(&A, &B, &C)` which is actually tough to do. `&(A,B,C) != (&A, &B, &C)`

2 Options came to mind to potentially skirt this:

#### Make our own signal container that has this built-in

A tuple is the only way I know of to refer to a collection of a variable number of values of heterogenous types. The only way around this would be to 
define multiple structs for each size of Input/Output:
```rust
pub struct Signal1<A: ?Sized> { ... }
pub struct Signal2<A: ?Sized, B: ?Sized> { ... }
pub struct Signal3<A: ?Sized, B: ?Sized, C: ?Sized> { ... }
```

The upshot is this would allow us to specify the `?Sized` trait bound, removing the implied `Sized` constraint. So it would be valid to write
a type like `Signal2<f64,[u8]>`.

This would somewhat ballon the complexity of our already complex machinery to handle variable tuple sizes.

#### Refer to types by reference everywhere

The core issue is that we can't easily bounce between value and reference types. If we made all types in block generics and traits and what have you be reference
types we could potentially side step this, i.e.
```rust
let sum_block = SumBlock::<(&f64, &Matrix<4,5,f64>, &f64)>::default();
```

This I think would potentially make the block trait/API much less strict and more difficult to reason about or codegen for. In this world the `ProcessBlock` trait would have to 
look something like this:
```rust
pub trait ProcessBlock: Default {
    type Inputs: Pass;
    type Output: Pass;
    type Parameters;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        context: &dyn Context,
        inputs: Self::Inputs,
    ) -> Self::Outputs;
}
```

So there would be no major guard rails besides testing and convention to make sure that blocks always take as input or output a value vs. a ref. 
One could imagine one block declaring `type Output = (&Matrix<2,3,f64>, f64)` while another block declares its input as `type Input = (&f64, &f64)`
where one of those `f64` values is coming from the first. We'd either need to enforce by norms and PRs that we always do ref or no ref, or we'd have 
to have the codegen detect this situation and add a `*` or `&` as appropriate.

Also lifetimes would begin to show up in a lot of places.

#### Keeping Passby?

Neither of these approaches is obviously great or simpler, or super easy to transition the existing blocks to. I think keeping Pass and PAssBy round may actually be the right move. It
works, we have found patterns to address that sharp edges with things like the `Apply` trait. To transition to one of these other approaches we would probably have to doa similar amount of
finding patterns to handle edge cases.

## Model Interop

Need I/O answer to turn model Input and Output structs, enums, etc. into signals for the blocks and then vice-versa

## Data Store Interop

Had one branch where I played with the idea of using `Deref<A>`, and `DerefMut<A>` instead of `&A` and `&mut A`. And further having the block traits
not actually return anything from `step()` instead accepting a `Deref<>` of inputs and a `DerefMut` of outputs. (Again the tuple thing puts us in a bind here, 
`Pass` would have to be used to get `(Deref<A>, Deref<B>)` instead of a Deref of the whole tuple).

The big upshot of this is that these are the traits that smart pointers, mutexes, RWLocks, etc. all make use of. So this could allow the actual return data to live
in static memory, the heap, the stack, wherever. 


