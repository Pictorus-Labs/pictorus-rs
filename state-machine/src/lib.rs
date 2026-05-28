#![no_std]

use enum_map::{EnumArray, EnumMap, enum_map};

pub trait StateMachineSpec {
    type States: Default + Copy + PartialEq;
    type Transitions: Copy;

    fn edge_lookup(
        current_state: Self::States,
        transition: Self::Transitions,
    ) -> Option<Self::States>;
}

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

    pub fn transition(&mut self, transition: SMS::Transitions) -> bool {
        if let Some(next_state) = SMS::edge_lookup(self.current_state, transition) {
            self.current_state = next_state;
            self.last_transition = Some(transition);
            true
        } else {
            false
        }
    }

    pub fn current_state(&self) -> SMS::States {
        self.current_state
    }

    pub fn last_transition(&self) -> Option<SMS::Transitions> {
        self.last_transition
    }

    pub fn reset(&mut self) {
        self.current_state = SMS::States::default();
        self.last_transition = None;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MachineSnapshot<S, T> {
    pub state: S,
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

impl<SMS: StateMachineSpec> StateMachine<SMS> {
    pub fn snapshot(&self) -> MachineSnapshot<SMS::States, SMS::Transitions> {
        MachineSnapshot {
            state: self.current_state,
            last_transition: self.last_transition,
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
    type Input;
    type Output: Default;

    fn tick(&mut self, input: &Self::Input, output: &mut Self::Output);
    fn reset(&mut self);

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

// Etc for (N1, N2, N3) and so on — could be generated with a macro

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

pub type LeafNode<SMS, I, O> = Node<SMS, NoChildren<I, O>>;

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
