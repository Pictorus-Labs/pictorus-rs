#![no_std]

pub mod corelib_wrapper;

use enum_map::{EnumArray, EnumMap, enum_map};

/// This is the core trait that is used to define a state machine.
/// Specifically it defines the states, transitions, and edges of the state machine.
/// This is an atomic state machine and does not include any hierarchical or parallel composition semantics.
pub trait StateMachineSpec {
    /// An enum representing the states of the state machine. Must implement `Default` to specify the initial state.
    type States: Default + Copy + PartialEq;
    /// An enum representing the transitions of the state machine. Must be `Copy` to allow for easy handling of transitions.
    type Transitions: Copy + PartialOrd;

    /// A function that defines the edges of the state machine. Given a current state and a transition,
    /// it returns the next state if the transition is valid from the current state, or `None` if the transition is invalid.
    fn edge_lookup(
        current_state: Self::States,
        transition: Self::Transitions,
    ) -> Option<Self::States>;
}

/// A simple atomic state machine implementation that uses a `StateMachineSpec` to define its behavior.
/// It keeps track of the current state and the last transition that was applied.
pub struct StateMachine<SMS: StateMachineSpec> {
    current_state: SMS::States,
    last_transition: Option<SMS::Transitions>,
}

impl<SMS: StateMachineSpec> Default for StateMachine<SMS> {
    fn default() -> Self {
        Self {
            current_state: SMS::States::default(),
            last_transition: None,
        }
    }
}

impl<SMS: StateMachineSpec> StateMachine<SMS> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Attempts to apply a transition to the state machine. If the transition is valid from the current state,
    /// the state machine updates its current state and last transition, and returns `true`. If the transition is invalid,
    /// the state machine remains unchanged and returns `false`.
    pub fn transition(&mut self, transition: SMS::Transitions) -> bool {
        if let Some(next_state) = SMS::edge_lookup(self.current_state, transition) {
            self.current_state = next_state;
            self.last_transition = Some(transition);
            true
        } else {
            false
        }
    }

    /// Accessor for the current state of the state machine.
    pub fn current_state(&self) -> SMS::States {
        self.current_state
    }

    /// Accessor for the last transition that was applied to the state machine, if any.
    pub fn last_transition(&self) -> Option<SMS::Transitions> {
        self.last_transition
    }

    /// Resets the state machine to its initial state and clears the last transition.
    pub fn reset(&mut self) {
        self.current_state = SMS::States::default();
        self.last_transition = None;
    }

    pub fn snapshot(&self) -> MachineSnapshot<SMS::States, SMS::Transitions> {
        MachineSnapshot {
            state: self.current_state,
            last_transition: self.last_transition,
        }
    }
}

/// A snapshot of the state machine's current state and last transition, used for communication with child nodes in a hierarchical state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MachineSnapshot<S, T> {
    // The current state of the state machine.
    pub state: S,
    // The last transition that was applied to the state machine, if any.
    pub last_transition: Option<T>,
}

impl<S: Default, T> Default for MachineSnapshot<S, T> {
    fn default() -> Self {
        Self {
            state: S::default(),
            last_transition: None,
        }
    }
}

/// A composable element of a hierarchical/parallel state machine tree.
///
/// `tick` reads the flat input bundle, advances internal state, and writes
/// the active machines' snapshots into `output`. Inactive machines do not
/// touch `output` — callers initialize it to `Default::default()` so inactive
/// entries naturally appear as default state with no last transition.
///
/// Activation semantics: when a parent transitions into a state with children
/// this tick, those children are NOT ticked this round. They become eligible
/// to consume transitions on the following tick. Conversely, children whose
/// parent state was just exited are `reset()` immediately so their snapshot
/// reflects their default the same tick.
pub trait NodeInterface {
    /// The type of the input bundle consumed by this node.
    /// This can and usually will include fields for other nodes (children, siblings, or ancestors)
    /// so the correct field must be used by the node's `tick` implementation.
    type Input;
    /// The type of the output bundle used by this node. This like the [`Input`] type
    /// can and usually will include fields for other nodes, so the correct field must be used by the node's `tick` implementation.
    type Output: Default;

    /// Advance the node's internal state based on the provided input, and write the node's snapshot to the output.
    /// It must only read from and write to the fields of `input` and `output` that are relevant to this node
    fn tick(&mut self, input: &Self::Input, output: &mut Self::Output);
    /// Reset the node's internal state to its default state
    fn reset(&mut self);

    /// A convenience method that combines `tick` with default output initialization
    /// Does not need to be implemented and should only be called at the top Root level of the state machine tree
    /// All nodes below the Root will have their snapshots written to the output by their parents' `tick` implementations,
    ///  so they do not need to be ticked directly
    fn step(&mut self, input: &Self::Input) -> Self::Output {
        let mut output = Self::Output::default();
        self.tick(input, &mut output);
        output
    }
}

impl<N1: NodeInterface, N2: NodeInterface<Input = N1::Input, Output = N1::Output>> NodeInterface
    for (N1, N2)
{
    type Input = N1::Input;
    type Output = N1::Output;

    fn tick(&mut self, input: &Self::Input, output: &mut Self::Output) {
        let (ref mut n1, ref mut n2) = *self;
        let ref mut o = *output;

        n1.tick(input, o);
        n2.tick(input, o);
    }

    fn reset(&mut self) {
        let (ref mut n1, ref mut n2) = *self;
        n1.reset();
        n2.reset();
    }
}

impl<
    N1: NodeInterface,
    N2: NodeInterface<Input = N1::Input, Output = N1::Output>,
    N3: NodeInterface<Input = N1::Input, Output = N1::Output>,
> NodeInterface for (N1, N2, N3)
{
    type Input = N1::Input;
    type Output = N1::Output;

    fn tick(&mut self, input: &Self::Input, output: &mut Self::Output) {
        let (ref mut n1, ref mut n2, ref mut n3) = *self;
        let ref mut o = *output;

        n1.tick(input, o);
        n2.tick(input, o);
        n3.tick(input, o);
    }

    fn reset(&mut self) {
        let (ref mut n1, ref mut n2, ref mut n3) = *self;
        n1.reset();
        n2.reset();
        n3.reset();
    }
}

// Etc for (N1, N2, N3, N4) and so on... could be generated with a macro

/// A node in a hierarchical state machine.
///
/// `SMS` is the `StateMachineSpec` that defines the state machine logic for this node, and `C` is an `EnumMap` of child nodes,
/// where each child corresponds to a state of the state machine.
///
/// The `input_filter` function is used to retrieve this node's state machine's transition input from the trait Input type passed in
/// (which is a flat bundle of inputs for every state machine in the tree). Similarly, the `output_mapper` function is used to write
/// this node's snapshot to the trait Output type (which is a flat bundle of snapshots for every state machine in the tree).
pub struct Node<SMS: StateMachineSpec, C>
where
    <SMS as StateMachineSpec>::States: EnumArray<C>,
    C: NodeInterface,
{
    machine: StateMachine<SMS>,
    children: EnumMap<SMS::States, C>,
    input_filter: fn(&C::Input) -> Option<SMS::Transitions>,
    output_mapper: fn(&mut C::Output, MachineSnapshot<SMS::States, SMS::Transitions>),
}

impl<SMS: StateMachineSpec, C> Node<SMS, C>
where
    <SMS as StateMachineSpec>::States: EnumArray<C>,
    C: NodeInterface,
{
    /// Constructs a new `Node` with the given children, input filter, and output mapper.
    pub fn new(
        children: EnumMap<SMS::States, C>,
        input_filter: fn(&C::Input) -> Option<SMS::Transitions>,
        output_mapper: fn(&mut C::Output, MachineSnapshot<SMS::States, SMS::Transitions>),
    ) -> Self {
        Self {
            machine: StateMachine::default(),
            children,
            input_filter,
            output_mapper,
        }
    }
}

impl<SMS: StateMachineSpec, I, O> Node<SMS, NoChildren<I, O>>
where
    <SMS as StateMachineSpec>::States: EnumArray<NoChildren<I, O>>,
    O: Default,
{
    /// A convenience constructor for leaf nodes that have no children. It automatically fills the `children` field with `NoChildren` variants.
    pub fn new_leaf(
        input_filter: fn(&I) -> Option<SMS::Transitions>,
        output_mapper: fn(&mut O, MachineSnapshot<SMS::States, SMS::Transitions>),
    ) -> Self {
        Self {
            machine: StateMachine::default(),
            children: enum_map! { _ => NoChildren::default() },
            input_filter,
            output_mapper,
        }
    }
}

impl<SMS: StateMachineSpec, C> NodeInterface for Node<SMS, C>
where
    <SMS as StateMachineSpec>::States: EnumArray<C>,
    C: NodeInterface,
    C::Output: Default,
{
    type Input = C::Input;
    type Output = C::Output;

    fn tick(&mut self, input: &Self::Input, output: &mut Self::Output) {
        let curr_state = self.machine.current_state();
        self.machine.last_transition = None; // Clear the last transition before processing input
        if let Some(transition) = (self.input_filter)(input) {
            self.machine.transition(transition);
        }
        let snapshot = self.machine.snapshot();
        (self.output_mapper)(output, snapshot);

        if curr_state == snapshot.state {
            self.children[snapshot.state].tick(input, output);
        } else {
            // State changed, so reset the child of the old state (if any) to reflect the snapshot
            self.children[curr_state].reset();
        }
    }

    fn reset(&mut self) {
        self.machine.reset();
        for child in self.children.values_mut() {
            child.reset();
        }
    }
}

/// A type alias for a leaf node, which is a `Node` with `NoChildren`. This is a common case and the alias provides a convenient shorthand.
pub type LeafNode<SMS, I, O> = Node<SMS, NoChildren<I, O>>;

/// A struct representing the absence of children for a node. This is used as the `C` parameter of `Node` for leaf nodes.
#[derive(Debug, Clone, Copy)]
pub struct NoChildren<I, O>(core::marker::PhantomData<(I, O)>);
impl<I, O> Default for NoChildren<I, O> {
    fn default() -> Self {
        Self(core::marker::PhantomData)
    }
}

impl<I, O: Default> NodeInterface for NoChildren<I, O> {
    type Input = I;
    type Output = O;

    fn tick(&mut self, _input: &Self::Input, _output: &mut Self::Output) {}
    fn reset(&mut self) {}
}

/// Declare a per-state children enum + `NodeInterface` impl for use as the
/// `C` parameter of [`Node`]. Each variant maps a parent state to the child
/// node active in that state; an implicit `None` variant handles states with
/// no children. `Input` and `Output` are inferred from the first listed
/// child's `NodeInterface` impl, so every child type must share the same
/// `Input`/`Output`.
///
/// ```ignore
/// state_machine::children! {
///     pub enum FooChildren {
///         F2 => BazNode,
///         F3 => (QuxNode, QuuxNode),   // parallel children via tuple impl
///     }
/// }
/// ```
#[macro_export]
macro_rules! children {
    (
        $vis:vis enum $name:ident {
            $first_variant:ident => $first_child:ty
            $(, $variant:ident => $child:ty )*
            $(,)?
        }
    ) => {
        $vis enum $name {
            None,
            $first_variant($first_child),
            $( $variant($child), )*
        }

        impl $crate::NodeInterface for $name {
            type Input = <$first_child as $crate::NodeInterface>::Input;
            type Output = <$first_child as $crate::NodeInterface>::Output;

            fn tick(
                &mut self,
                input: &Self::Input,
                output: &mut Self::Output,
            ) {
                match self {
                    Self::None => {}
                    Self::$first_variant(n) => n.tick(input, output),
                    $( Self::$variant(n) => n.tick(input, output), )*
                }
            }

            fn reset(&mut self) {
                match self {
                    Self::None => {}
                    Self::$first_variant(n) => n.reset(),
                    $( Self::$variant(n) => n.reset(), )*
                }
            }
        }
    };
}

pub trait TransitionPrioritize {
    type Inner;
    fn prioritize(self, other: Self) -> Self;
    fn prioritize_val(self, enabled: bool, other: Self::Inner) -> Self;
}

impl<T: PartialOrd + Copy> TransitionPrioritize for Option<T> {
    type Inner = T;
    fn prioritize(self, other: Self) -> Self {
        match (self, other) {
            (Some(a), Some(b)) => Some(if a < b { a } else { b }),
            (x, None) | (None, x) => x,
        }
    }

    fn prioritize_val(self, enabled: bool, other: Self::Inner) -> Self {
        if enabled {
            self.prioritize(Some(other))
        } else {
            self
        }
    }
}
