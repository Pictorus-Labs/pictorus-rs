#![no_std]

#[cfg(any(feature = "std", test))]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

use enum_map::{EnumArray, EnumMap, enum_map};

/// Output Sink Trait
///
/// This defines an interface that can be used to emit output events
///
/// Usually the provided [`Events`] struct is sufficient. But the trait allows for customization as well as testing with a mock sink.
pub trait EventSink<E> {
    /// Emit an event. This is the only required method
    fn emit(&mut self, e: E);
    /// Emit an optional event. This is a convenience method that calls `emit` if the event is `Some`, and does nothing if it is `None`.
    fn emit_opt(&mut self, e: Option<E>) {
        if let Some(e) = e {
            self.emit(e)
        }
    }
}

/// The provided default sink implementation: counts of emitted events, plus an optional
/// log of their order that is enabled via the `event-log` feature (and requires `alloc`).
pub struct Events<E: EnumArray<u32> + Copy> {
    pub counts: EnumMap<E, u32>,
    #[cfg(feature = "event-log")]
    pub order: alloc::vec::Vec<E>,
}

impl<E: EnumArray<u32> + Copy> Events<E> {
    pub fn clear(&mut self) {
        self.counts = EnumMap::default();
        #[cfg(feature = "event-log")]
        self.order.clear();
    }
}

impl<E: EnumArray<u32> + Copy> Default for Events<E> {
    fn default() -> Self {
        Self {
            counts: EnumMap::default(),
            #[cfg(feature = "event-log")]
            order: alloc::vec::Vec::new(),
        }
    }
}

impl<E: EnumArray<u32> + Copy> EventSink<E> for Events<E> {
    fn emit(&mut self, e: E) {
        self.counts[e] += 1;
        #[cfg(feature = "event-log")]
        self.order.push(e);
    }
}

/// A Guard is a function that takes the machine's InputData and returns a boolean indicating whether a transition is allowed to occur
pub type Guard<D> = fn(&D) -> bool;
/// A composite type representing a transition edge, consisting of an optional guard, an optional output event, and an optional target state
/// where None indicates an "internal transition"
pub type GuardedEdge<D, O, S> = (Option<Guard<D>>, Option<O>, Option<S>);
/// A collection of edges for a specific event, consisting of the event and a slice of guarded edges, ordered by priority
pub type EventEdges<E, D, O, S> = (E, &'static [GuardedEdge<D, O, S>]);
/// A collection of edges for a specific state, consisting of the state and a slice of event edges
pub type StateEdges<S, E, D, O> = (S, &'static [EventEdges<E, D, O, S>]);
/// The entire set of edges for a state machine, represented as a slice of state edges
pub type EdgeTable<S, E, D, O> = &'static [StateEdges<S, E, D, O>]; // the whole machine

/// A simple struct representing a transition edge, consisting of an optional output event and a target state
pub struct Edge<S, Out> {
    pub action: Option<Out>,
    pub target: Option<S>,
}

/// Using an [`EdgeTable`], a source state, an event, and the current input data, resolve to the first valid outgoing edge (if any)
/// according to the transition rules: find the matching state slice, then the matching event slice, then return the first edge whose guard passes (or has no guard).
pub fn resolve_table<S, E, D, O>(
    table: EdgeTable<S, E, D, O>,
    state: S,
    event: E,
    data: &D,
) -> Option<Edge<S, O>>
where
    S: Copy + PartialEq,
    E: Copy + PartialEq,
    O: Copy,
{
    let by_event = table.iter().find(|(s, _)| *s == state)?.1;
    let edges = by_event.iter().find(|(e, _)| *e == event)?.1;
    for (guard, action, target) in edges {
        if guard.is_none_or(|g| g(data)) {
            return Some(Edge {
                action: *action,
                target: *target,
            });
        }
    }
    None
}

/// Core definition for a State Machine's structure and behavior
/// This represents a single atomic state machine, which can later be composed into a tree structure via
/// the `Node` struct and the `NodeInterface` trait to form parallel regions and hierarchical states.
pub trait StateMachineSpec {
    /// An enum representing the states of this machine (specific to this atomic machine)
    type State: Default + Copy + PartialEq + 'static;
    /// The enum of input events that can trigger transitions in this machine. This must be
    /// the same across every atomic machine that will be composed in to a single hierarchical state machine
    type InputEvent: Copy + PartialEq + 'static;
    /// The type of the input data that guards can read to make transition decisions. This must be
    /// the same across every atomic machine that will be composed in to a single hierarchical state machine
    type InputData: 'static;
    /// The enum of output events that this machine can emit on transitions and state entry/exit. This must be
    /// the same across every atomic machine that will be composed in to a single hierarchical state machine
    type OutputEvent: Copy + 'static;

    /// A static table that represents all inter-state transitions for this machine
    /// The default implementation of [`resolve`] looks up edges in this table. Custom behavior
    /// could be implemented by leaving this empty (setting it to `&[]`) and overriding [`resolve`] with a custom function
    const EDGES: EdgeTable<Self::State, Self::InputEvent, Self::InputData, Self::OutputEvent> = &[];

    /// Given a source state, an event, and the current input data, resolve to the first valid outgoing edge (if any) according
    /// to the transition rules: find the matching state slice, then the matching event slice, then return the first edge whose
    ///  guard passes (or has no guard). The default implementation looks up edges in the [`EDGES`] table. Custom behavior can be
    /// implemented by overriding this function.
    fn resolve(
        state: Self::State,
        event: Self::InputEvent,
        data: &Self::InputData,
    ) -> Option<Edge<Self::State, Self::OutputEvent>> {
        resolve_table(Self::EDGES, state, event, data)
    }

    /// Given a state, return an optional output event to emit on entry to that state. The default implementation returns `None` for every state.
    fn on_enter(_state: Self::State) -> Option<Self::OutputEvent> {
        None
    }

    /// Given a state, return an optional output event to emit on exit from that state. The default implementation returns `None` for every state.
    fn on_exit(_state: Self::State) -> Option<Self::OutputEvent> {
        None
    }

    /// Given a state, return an optional output event to emit during that state (i.e. on every step where the machine remains in that state).
    /// The default implementation returns `None` for every state.
    fn during(_state: Self::State) -> Option<Self::OutputEvent> {
        None
    }

    /// Return the default transition for this machine
    /// This is the transition that will be taken when this state machine first becomes active. It must specify the target (initial) state, and may
    /// optionally specify an action to emit on that initial transition. The default implementation returns a default-constructed target state and no action.
    fn default_transition() -> (Self::State, Option<Self::OutputEvent>) {
        (Self::State::default(), None)
    }
}

/// A simple atomic state machine implementation that uses a `StateMachineSpec` to define its behavior.
/// It keeps track of the current state and possibly a pending transition edge that has been selected but not yet executed.
pub struct Machine<SMS: StateMachineSpec> {
    current: SMS::State,
    pending: Option<Edge<SMS::State, SMS::OutputEvent>>,
}

impl<S: StateMachineSpec> Machine<S> {
    pub fn current(&self) -> S::State {
        self.current
    }
}

impl<S: StateMachineSpec> Default for Machine<S> {
    fn default() -> Self {
        Self {
            current: S::State::default(),
            pending: None,
        }
    }
}

/// Defines the interface for a composable element of a hierarchical/parallel state machine tree
pub trait NodeInterface {
    type InputEvent: Copy;
    type InputData;
    type OutputEvent: Copy;

    /// Returns whether this subtree selected a transition (that will now be pending execution)
    fn select(&mut self, event: Self::InputEvent, data: &Self::InputData) -> bool;
    /// Execute the pending transition if there is one, or the `during` event if not
    fn execute_pending<K: EventSink<Self::OutputEvent>>(&mut self, sink: &mut K);
    /// Perform the entry actions for this subtree, cascading down to defaults.
    /// This is called when this subtree becomes active due to a transition from its parent.
    fn enter<K: EventSink<Self::OutputEvent>>(&mut self, sink: &mut K);
    /// Perform the exit actions for this subtree, cascading up to defaults.
    /// This is called when this subtree becomes inactive due to a transition to its parent.
    fn exit<K: EventSink<Self::OutputEvent>>(&mut self, sink: &mut K);
    /// Reset the state of this subtree to its initial state.
    fn reset(&mut self);
}

impl<A, B> NodeInterface for (A, B)
where
    A: NodeInterface,
    B: NodeInterface<
            InputEvent = A::InputEvent,
            InputData = A::InputData,
            OutputEvent = A::OutputEvent,
        >,
{
    type InputEvent = A::InputEvent;
    type InputData = A::InputData;
    type OutputEvent = A::OutputEvent;

    fn select(&mut self, input: Self::InputEvent, data: &Self::InputData) -> bool {
        self.0.select(input, data) | self.1.select(input, data)
    }
    fn execute_pending<K: EventSink<Self::OutputEvent>>(&mut self, sink: &mut K) {
        self.0.execute_pending(sink);
        self.1.execute_pending(sink);
    }
    fn enter<K: EventSink<Self::OutputEvent>>(&mut self, sink: &mut K) {
        self.0.enter(sink);
        self.1.enter(sink);
    }
    fn exit<K: EventSink<Self::OutputEvent>>(&mut self, sink: &mut K) {
        self.0.exit(sink);
        self.1.exit(sink);
    }
    fn reset(&mut self) {
        self.0.reset();
        self.1.reset();
    }
}

impl<A, B, C> NodeInterface for (A, B, C)
where
    A: NodeInterface,
    B: NodeInterface<
            InputEvent = A::InputEvent,
            InputData = A::InputData,
            OutputEvent = A::OutputEvent,
        >,
    C: NodeInterface<
            InputEvent = A::InputEvent,
            InputData = A::InputData,
            OutputEvent = A::OutputEvent,
        >,
{
    type InputEvent = A::InputEvent;
    type InputData = A::InputData;
    type OutputEvent = A::OutputEvent;

    fn select(&mut self, input: Self::InputEvent, data: &Self::InputData) -> bool {
        self.0.select(input, data) | self.1.select(input, data) | self.2.select(input, data)
    }
    fn execute_pending<K: EventSink<Self::OutputEvent>>(&mut self, sink: &mut K) {
        self.0.execute_pending(sink);
        self.1.execute_pending(sink);
        self.2.execute_pending(sink);
    }
    fn enter<K: EventSink<Self::OutputEvent>>(&mut self, sink: &mut K) {
        self.0.enter(sink);
        self.1.enter(sink);
        self.2.enter(sink);
    }
    fn exit<K: EventSink<Self::OutputEvent>>(&mut self, sink: &mut K) {
        self.0.exit(sink);
        self.1.exit(sink);
        self.2.exit(sink);
    }
    fn reset(&mut self) {
        self.0.reset();
        self.1.reset();
        self.2.reset();
    }
}

// Etc. for larger tuples if desired, would be a simple macro if needed

/// A struct that implements [`NodeInterface`]
///
/// It contains a [`Machine`] that represents the current state and possibly a pending transition of this node,
/// as well as a set of child nodes for each state (representing the hierarchical/parallel structure of the state machine tree).
pub struct Node<SMS: StateMachineSpec, C>
where
    SMS::State: EnumArray<C>,
    C: NodeInterface<
            OutputEvent = SMS::OutputEvent,
            InputEvent = SMS::InputEvent,
            InputData = SMS::InputData,
        >,
{
    machine: Machine<SMS>,
    children: EnumMap<SMS::State, C>,
}

impl<SMS: StateMachineSpec, C> Node<SMS, C>
where
    SMS::State: EnumArray<C>,
    C: NodeInterface<
            OutputEvent = SMS::OutputEvent,
            InputEvent = SMS::InputEvent,
            InputData = SMS::InputData,
        >,
{
    /// Create a new `Node` with the given children. The machine will be initialized to its default state.
    pub fn new(children: EnumMap<SMS::State, C>) -> Self {
        Self {
            machine: Machine::default(),
            children,
        }
    }
    /// The active state of this node's own machine.
    pub fn state(&self) -> SMS::State {
        self.machine.current
    }

    /// The child subtree under the currently active state. Lets a caller that
    /// knows `C` (e.g. the machine's author) walk into the active branch.
    pub fn active_child(&self) -> &C {
        &self.children[self.machine.current]
    }

    /// The child subtree under an arbitrary state.
    pub fn child(&self, state: SMS::State) -> &C {
        &self.children[state]
    }
}

impl<SMS: StateMachineSpec> Node<SMS, NoChildren<SMS::InputEvent, SMS::InputData, SMS::OutputEvent>>
where
    SMS::State: EnumArray<NoChildren<SMS::InputEvent, SMS::InputData, SMS::OutputEvent>>,
{
    /// Create a new leaf `Node` with no children. The machine will be initialized to its default state.
    pub fn new_leaf() -> Self {
        Self {
            machine: Machine::default(),
            children: enum_map! { _ => NoChildren::default() },
        }
    }
}

impl<SMS: StateMachineSpec, C> NodeInterface for Node<SMS, C>
where
    SMS::State: EnumArray<C>,
    C: NodeInterface<
            OutputEvent = SMS::OutputEvent,
            InputEvent = SMS::InputEvent,
            InputData = SMS::InputData,
        >,
{
    type InputEvent = SMS::InputEvent;
    type InputData = SMS::InputData;
    type OutputEvent = SMS::OutputEvent;

    fn select(&mut self, event: Self::InputEvent, data: &Self::InputData) -> bool {
        let s = self.machine.current;

        // Child-first: a deeper handler preempts this level entirely.
        if self.children[s].select(event, data) {
            return true;
        }

        // Nobody below handled it — try our own outgoing edges.
        if let Some(edge) = SMS::resolve(s, event, data) {
            self.machine.pending = Some(edge);
            return true;
        }

        false
    }

    fn execute_pending<K: EventSink<Self::OutputEvent>>(&mut self, sink: &mut K) {
        let pending = self.machine.pending.take();
        // May be None if pending was none or if it is an internal transition
        let target_state = pending.as_ref().and_then(|e| e.target);
        // May be None if pending was none or if the edge has no action
        let transition_action = pending.as_ref().and_then(|e| e.action);

        if let Some(target_state) = target_state {
            // Need to do a full exit and enter sequence
            let old = self.machine.current;

            self.children[old].exit(sink); // deeper exit actions first
            sink.emit_opt(SMS::on_exit(old)); // then this state's own
            self.children[old].reset(); // clean for next activation

            sink.emit_opt(transition_action); // transition action

            self.machine.current = target_state;
            sink.emit_opt(SMS::on_enter(target_state));
            self.children[target_state].enter(sink); // cascade defaults ↓
        } else {
            // Internal transition or No transition, the only difference is that `transition_action`
            // may have been set if it is an internal transition, emit_opt will handle the None case correctly.
            let s = self.machine.current;
            sink.emit_opt(SMS::during(s));
            sink.emit_opt(transition_action);
            self.children[s].execute_pending(sink);
        }
    }

    fn enter<K: EventSink<Self::OutputEvent>>(&mut self, sink: &mut K) {
        // Activating from default transition
        let (default_state, default_action) = SMS::default_transition();
        sink.emit_opt(default_action);
        self.machine.current = default_state;
        sink.emit_opt(SMS::on_enter(default_state));
        self.children[default_state].enter(sink);
    }

    fn exit<K: EventSink<Self::OutputEvent>>(&mut self, sink: &mut K) {
        let s = self.machine.current;
        self.children[s].exit(sink); // bottom-up
        sink.emit_opt(SMS::on_exit(s));
    }

    fn reset(&mut self) {
        self.machine = Machine::default();
        for c in self.children.values_mut() {
            c.reset();
        }
    }
}

/// A type alias for a leaf node, which is a `Node` with `NoChildren`. This is a common case and the alias provides a convenient shorthand.
pub type LeafNode<SMS, IE, ID, O> = Node<SMS, NoChildren<IE, ID, O>>;

/// A struct representing the absence of children for a node. This is used as the `C` parameter of `Node` for leaf nodes.
pub struct NoChildren<IE: Copy, ID, Out>(core::marker::PhantomData<(IE, ID, Out)>);
impl<IE: Copy, ID, Out> Default for NoChildren<IE, ID, Out> {
    fn default() -> Self {
        Self(core::marker::PhantomData)
    }
}
impl<IE: Copy, ID, Out: Copy> NodeInterface for NoChildren<IE, ID, Out> {
    type InputEvent = IE;
    type InputData = ID;
    type OutputEvent = Out;
    fn select(&mut self, _: Self::InputEvent, _: &Self::InputData) -> bool {
        false
    }
    fn execute_pending<K: EventSink<Self::OutputEvent>>(&mut self, _: &mut K) {}
    fn enter<K: EventSink<Self::OutputEvent>>(&mut self, _: &mut K) {}
    fn exit<K: EventSink<Self::OutputEvent>>(&mut self, _: &mut K) {}
    fn reset(&mut self) {}
}

/// Define a parent state's per-child enum and its [`NodeInterface`] forwarding impl in one shot.
///
/// A composite [`Node`]'s children are held in an `EnumMap<State, C>`, so every state must map to
/// the *same* type `C`. When some states nest a machine and others don't, `C` has to be a hand-rolled
/// enum that wraps each distinct subtree and forwards all five `NodeInterface` methods. That impl is
/// pure boilerplate — this macro writes it for you.
///
/// List only the states that nest a child machine, each as `Variant => ChildNodeType`. Every other
/// (leaf) state is covered by a generated `Leaf` variant holding [`NoChildren`], constructed via the
/// generated `leaf()` associated function. In the `EnumMap`, map the leaf states with `enum_map!`'s
/// `_` catch-all:
///
/// ```ignore
/// children! {
///     pub enum NavChildren {
///         Cruising => SpeedNode,   // Cruising nests the L3 Speed machine
///     }                            // Hovering (and any other state) -> Leaf(NoChildren)
/// }
///
/// let nav = Node::<NavSpec, NavChildren>::new(enum_map! {
///     Nav::Cruising => NavChildren::Cruising(Node::new_leaf()),
///     _             => NavChildren::leaf(),
/// });
/// ```
///
/// At least one composite variant is required: its node type supplies the shared
/// `InputEvent` / `InputData` / `OutputEvent` for the enum and the `Leaf` variant.
#[macro_export]
macro_rules! children {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $first_variant:ident => $first_node:ty
            $(, $variant:ident => $node:ty )*
            $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis enum $name {
            None,
            $first_variant($first_node),
            $( $variant($node), )*
        }

        impl $crate::NodeInterface for $name {
            type InputEvent = <$first_node as $crate::NodeInterface>::InputEvent;
            type InputData = <$first_node as $crate::NodeInterface>::InputData;
            type OutputEvent = <$first_node as $crate::NodeInterface>::OutputEvent;

            fn select(&mut self, event: Self::InputEvent, data: &Self::InputData) -> bool {
                match self {
                    Self::None => false,
                    Self::$first_variant(c) => c.select(event, data),
                    $( Self::$variant(c) => c.select(event, data), )*

                }
            }
            fn execute_pending<K: $crate::EventSink<Self::OutputEvent>>(&mut self, sink: &mut K) {
                match self {
                    Self::None => {},
                    Self::$first_variant(c) => c.execute_pending(sink),
                    $( Self::$variant(c) => c.execute_pending(sink), )*
                }
            }
            fn enter<K: $crate::EventSink<Self::OutputEvent>>(&mut self, sink: &mut K) {
                match self {
                    Self::None => {},
                    Self::$first_variant(c) => c.enter(sink),
                    $( Self::$variant(c) => c.enter(sink), )*

                }
            }
            fn exit<K: $crate::EventSink<Self::OutputEvent>>(&mut self, sink: &mut K) {
                match self {
                    Self::None => {},
                    Self::$first_variant(c) => c.exit(sink),
                    $( Self::$variant(c) => c.exit(sink), )*
                }
            }
            fn reset(&mut self) {
                match self {
                    Self::None => {},
                    Self::$first_variant(c) => c.reset(),
                    $( Self::$variant(c) => c.reset(), )*
                }
            }
        }
    };
}

/// The primary runtime interface for users of this library: a `StateMachineRoot` is a wrapper around the top-level
/// `Node` that provides a simple API for stepping the machine with input events and data, and for creating the machine with an initial node and sink.
pub struct StateMachineRoot<N: NodeInterface> {
    node: N,
}

impl<N: NodeInterface> StateMachineRoot<N> {
    pub fn create(node: N, sink: &mut impl EventSink<N::OutputEvent>) -> Self {
        let mut root = Self { node };
        root.node.enter(sink);
        root
    }

    pub fn step(
        &mut self,
        input_event: N::InputEvent,
        input_data: &N::InputData,
        sink: &mut impl EventSink<N::OutputEvent>,
    ) {
        self.node.select(input_event, input_data);
        self.node.execute_pending(sink);
    }
    /// Read-only access to the root node, for inspection.
    pub fn root(&self) -> &N {
        &self.node
    }
}
impl<N: NodeInterface> StateMachineRoot<N>
where
    N::InputEvent: EnumArray<bool>,
{
    pub fn execute(
        &mut self,
        input_events: EnumMap<N::InputEvent, bool>,
        input_data: &N::InputData,
        sink: &mut impl EventSink<N::OutputEvent>,
    ) {
        for (event, should_fire) in input_events {
            if should_fire {
                self.step(event, input_data, sink);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use enum_map::{Enum, enum_map};

    // ── Output events ────────────────────────────────────────────────────
    // Every action in the spec walkthrough, named after the action it stands
    // for. (S1.init / S2.init — the default-transition actions — have no
    // variant: `enter` does not emit them yet, see the note below.)
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
    enum Out {
        A1Init,           // Active::Audio  default transition
        PlayingExit,      // Active::Audio  on_exit
        A2Init,           // Active::Screen default transition
        ShowingVideoExit, // Active::Screen on_exit
        ActiveExit,       // Active         on_exit
        TEffect,          // Active → Standby transition action
        StandbyEntry,     // Standby        on_enter
        S1Init,           // Standby::Power   default transition
        LowPowerEntry,    // Standby::Power   on_enter
        S2Init,           // Standby::Network default transition
        ListeningEntry,   // Standby::Network on_enter
        // exercised by the second test (during + descendant transition)
        StandbyDuring,
        LowPowerDuring,
        ListeningExit,
        NetworkConnect,
        ConnectedEntry,
        // Exercised by the third test (during + internal transition)
        InternalAct,
        IdleDuring,
    }

    // ── States, one enum per region ──────────────────────────────────────
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Default)]
    enum Top {
        #[default]
        Active,
        Standby,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Default)]
    enum Audio {
        #[default]
        Playing,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Default)]
    enum Screen {
        #[default]
        ShowingVideo,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Default)]
    enum Power {
        #[default]
        LowPower,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Default)]
    enum Network {
        #[default]
        Listening,
        Connected,
    }

    // ── Shared event + data + input bundle ───────────────────────────────
    #[derive(Clone, Copy, PartialEq)]
    enum Ev {
        Sleep,
        Tick,
    }

    struct Data {
        battery_ok: bool,
    }
    struct Input {
        event: Ev,
        data: Data,
    }

    // ── Specs ─────────────────────────────────────────────────────────────
    // The top machine: the only non-trivial transition table in the example.
    // Note the guard reads Data and the slice order is the transition priority.
    struct TopSpec;
    impl StateMachineSpec for TopSpec {
        type State = Top;
        type InputEvent = Ev;
        type InputData = Data;
        type OutputEvent = Out;

        const EDGES: EdgeTable<Top, Ev, Data, Out> = &[
            (
                Top::Active,
                &[(
                    Ev::Sleep,
                    &[
                        // CheckAlarms-style edge: sleep[batteryOK]/T.effect
                        (
                            Some(|d: &Data| d.battery_ok),
                            Some(Out::TEffect),
                            Some(Top::Standby),
                        ),
                    ],
                )],
            ),
            // Standby has no outgoing edges in this slice of the example.
        ];

        fn on_exit(s: Top) -> Option<Out> {
            match s {
                Top::Active => Some(Out::ActiveExit),
                _ => None,
            }
        }
        fn on_enter(s: Top) -> Option<Out> {
            match s {
                Top::Standby => Some(Out::StandbyEntry),
                _ => None,
            }
        }
        fn during(s: Top) -> Option<Out> {
            match s {
                Top::Standby => Some(Out::StandbyDuring),
                _ => None,
            }
        }
    }

    struct AudioSpec;
    impl StateMachineSpec for AudioSpec {
        type State = Audio;
        type InputEvent = Ev;
        type InputData = Data;
        type OutputEvent = Out;
        fn on_exit(_: Audio) -> Option<Out> {
            Some(Out::PlayingExit)
        }
        fn default_transition() -> (Self::State, Option<Self::OutputEvent>) {
            (Audio::Playing, Some(Out::A1Init))
        }
    }

    struct ScreenSpec;
    impl StateMachineSpec for ScreenSpec {
        type State = Screen;
        type InputEvent = Ev;
        type InputData = Data;
        type OutputEvent = Out;
        fn on_exit(_: Screen) -> Option<Out> {
            Some(Out::ShowingVideoExit)
        }
        fn default_transition() -> (Self::State, Option<Self::OutputEvent>) {
            (Screen::ShowingVideo, Some(Out::A2Init))
        }
    }

    struct PowerSpec;
    impl StateMachineSpec for PowerSpec {
        type State = Power;
        type InputEvent = Ev;
        type InputData = Data;
        type OutputEvent = Out;
        fn on_enter(_: Power) -> Option<Out> {
            Some(Out::LowPowerEntry)
        }
        fn during(_: Power) -> Option<Out> {
            Some(Out::LowPowerDuring)
        }
        fn default_transition() -> (Self::State, Option<Self::OutputEvent>) {
            (Power::LowPower, Some(Out::S1Init))
        }
    }

    struct NetworkSpec;
    impl StateMachineSpec for NetworkSpec {
        type State = Network;
        type InputEvent = Ev;
        type InputData = Data;
        type OutputEvent = Out;

        // A region-internal transition, used to show `during` still fires on an
        // ancestor whose descendant transitions in the same step.
        const EDGES: EdgeTable<Network, Ev, Data, Out> = &[(
            Network::Listening,
            &[(
                Ev::Tick,
                &[(None, Some(Out::NetworkConnect), Some(Network::Connected))],
            )],
        )];

        fn on_enter(s: Network) -> Option<Out> {
            match s {
                Network::Listening => Some(Out::ListeningEntry),
                Network::Connected => Some(Out::ConnectedEntry),
            }
        }
        fn on_exit(s: Network) -> Option<Out> {
            match s {
                Network::Listening => Some(Out::ListeningExit),
                _ => None,
            }
        }
        fn default_transition() -> (Self::State, Option<Self::OutputEvent>) {
            (Network::Listening, Some(Out::S2Init))
        }
    }

    // ── Tree shape ────────────────────────────────────────────────────────
    // Active's regions and Standby's regions are *different* tuple types, so
    // the per-state children of the top machine can't be one homogeneous `C`.
    // This enum is the unifying `C` — the small tax the EnumMap<State, C> model
    // charges for heterogeneous subtrees.
    type Leaf<S> = LeafNode<S, Ev, Data, Out>;

    enum TopChildren {
        ActiveKids((Leaf<AudioSpec>, Leaf<ScreenSpec>)), // Audio ∥ Screen
        StandbyKids((Leaf<PowerSpec>, Leaf<NetworkSpec>)), // Power ∥ Network
    }

    impl NodeInterface for TopChildren {
        type InputEvent = Ev;
        type InputData = Data;
        type OutputEvent = Out;
        fn select(&mut self, input_event: Self::InputEvent, input_data: &Self::InputData) -> bool {
            match self {
                TopChildren::ActiveKids(t) => t.select(input_event, input_data),
                TopChildren::StandbyKids(t) => t.select(input_event, input_data),
            }
        }
        fn execute_pending<K: EventSink<Out>>(&mut self, sink: &mut K) {
            match self {
                TopChildren::ActiveKids(t) => t.execute_pending(sink),
                TopChildren::StandbyKids(t) => t.execute_pending(sink),
            }
        }
        fn enter<K: EventSink<Out>>(&mut self, sink: &mut K) {
            match self {
                TopChildren::ActiveKids(t) => t.enter(sink),
                TopChildren::StandbyKids(t) => t.enter(sink),
            }
        }
        fn exit<K: EventSink<Out>>(&mut self, sink: &mut K) {
            match self {
                TopChildren::ActiveKids(t) => t.exit(sink),
                TopChildren::StandbyKids(t) => t.exit(sink),
            }
        }
        fn reset(&mut self) {
            match self {
                TopChildren::ActiveKids(t) => t.reset(),
                TopChildren::StandbyKids(t) => t.reset(),
            }
        }
    }

    type TopNode = Node<TopSpec, TopChildren>;

    fn build() -> TopNode {
        Node {
            machine: Machine::default(),
            children: enum_map! {
                Top::Active  => TopChildren::ActiveKids((Node::new_leaf(),  Node::new_leaf())),
                Top::Standby => TopChildren::StandbyKids((Node::new_leaf(), Node::new_leaf())),
            },
        }
    }

    // ── Order-preserving sink (no alloc) ─────────────────────────────────────
    struct RecordingSink<O: Copy> {
        log: [Option<O>; 16],
        len: usize,
    }
    impl<O: Copy> RecordingSink<O> {
        fn new() -> Self {
            Self {
                log: [None; 16],
                len: 0,
            }
        }
        fn events(&self) -> &[Option<O>] {
            &self.log[..self.len]
        }
        fn clear(&mut self) {
            self.len = 0;
        }
    }
    impl<O: Copy> EventSink<O> for RecordingSink<O> {
        fn emit(&mut self, e: O) {
            self.log[self.len] = Some(e);
            self.len += 1;
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    #[test]
    fn sleep_walkthrough_matches_spec() {
        // Initial configuration: <Active – (Playing, ShowingVideo)>, established
        // by the #[default] of every region — no explicit enter needed.
        let root = build();
        let input = Input {
            event: Ev::Sleep,
            data: Data { battery_ok: true },
        };

        let mut sink = RecordingSink::new();
        let mut root_sm = StateMachineRoot::create(root, &mut sink);
        assert_eq!(sink.events(), &[Some(Out::A1Init), Some(Out::A2Init)]);
        sink.clear();

        root_sm.step(input.event, &input.data, &mut sink);

        // Spec order: exit source bottom-up, transition action, enter target
        // top-down. The two pairs marked "region order" are emitted in tuple
        // order; the spec leaves intra-level region order undefined, so the
        // test pins the implementation's deterministic choice.
        let expected: &[Option<Out>] = &[
            Some(Out::PlayingExit),      // ┐ region order (Audio before Screen)
            Some(Out::ShowingVideoExit), // ┘
            Some(Out::ActiveExit),       // source fully exited
            Some(Out::TEffect),          // transition action, between exit & enter
            Some(Out::StandbyEntry),     // target entered, top-down…
            Some(Out::S1Init),           // …first the Power region's default transition…
            Some(Out::LowPowerEntry),    // ┐ region order (Power before Network)
            Some(Out::S2Init),           // …then the Network region's default transition…
            Some(Out::ListeningEntry),   // ┘ …down to a stable leaf per region
        ];
        assert_eq!(sink.events(), expected);

        // Resulting stable configuration: <Standby – (LowPower, Listening)>.
        assert_eq!(root_sm.node.machine.current, Top::Standby);
        match &root_sm.node.children[Top::Standby] {
            TopChildren::StandbyKids((power, network)) => {
                assert_eq!(power.machine.current, Power::LowPower);
                assert_eq!(network.machine.current, Network::Listening);
            }
            _ => unreachable!(),
        }

        // Child-first selection: neither region handled `sleep`, so the
        // composite fired its own edge. (If a region *had* a sleep edge it would
        // have preempted this — that's the deeper-handler-wins rule.)
    }

    #[test]
    fn during_fires_even_when_a_descendant_transitions() {
        let mut sink = RecordingSink::new();
        // Boot into <Standby – (LowPower, Listening)> via the validated sleep step.
        let mut root_sm = StateMachineRoot::create(build(), &mut sink);

        root_sm.step(
            Ev::Sleep,
            &Data { battery_ok: true },
            &mut RecordingSink::new(),
        );

        // A Tick: the Network region transitions Listening → Connected, while
        // Standby and the Power region merely persist.
        let mut sink = RecordingSink::new();
        root_sm.step(Ev::Tick, &Data { battery_ok: true }, &mut sink);

        let expected: &[Option<Out>] = &[
            Some(Out::StandbyDuring),  // ancestor `during` STILL fires…
            Some(Out::LowPowerDuring), // …as does the persisting region's
            // Network region transitions; it does NOT fire `during`:
            Some(Out::ListeningExit),
            Some(Out::NetworkConnect),
            Some(Out::ConnectedEntry),
        ];
        assert_eq!(sink.events(), expected);

        match &root_sm.node.children[Top::Standby] {
            TopChildren::StandbyKids((power, network)) => {
                assert_eq!(power.machine.current, Power::LowPower); // persisted
                assert_eq!(network.machine.current, Network::Connected); // moved
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn during_fires_before_internal_transition_action() {
        // Internal transition = edge with target None. Confirms that
        // `during` fires on an internal-transition step, and BEFORE the
        // internal action.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Default)]
        enum States {
            #[default]
            Idle,
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
        enum Events {
            Tick,
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
        enum Out {
            IdleDuring,
            InternalAct,
        }

        struct Spec;
        impl StateMachineSpec for Spec {
            type State = States;
            type InputEvent = Events;
            type InputData = Data;
            type OutputEvent = Out;

            // Tick → internal transition (None target), emitting InternalAct.
            const EDGES: EdgeTable<States, Events, Data, Out> = &[(
                States::Idle,
                &[(Events::Tick, &[(None, Some(Out::InternalAct), None)])],
            )];

            fn during(_: States) -> Option<Out> {
                Some(Out::IdleDuring)
            }
        }

        let mut sink = RecordingSink::new();
        let mut sm = StateMachineRoot::create(
            Node::<Spec, NoChildren<Events, Data, Out>>::new_leaf(),
            &mut sink,
        );
        sink.clear(); // create emits nothing here, but be explicit

        sm.step(Events::Tick, &Data { battery_ok: true }, &mut sink);

        // during BEFORE action; no on_enter/on_exit; state unchanged.
        assert_eq!(
            sink.events(),
            &[Some(Out::IdleDuring), Some(Out::InternalAct)],
        );
        assert_eq!(sm.node.machine.current, States::Idle);
    }

    #[test]
    fn children_macro_forwards_and_cascades() {
        // A two-level machine assembled with `children!`: the parent `Busy`
        // state nests a child `Work` machine, while `Idle` is a leaf handled by
        // the generated `Leaf` variant. This exercises the generated enum, its
        // `leaf()` constructor, and every forwarded `NodeInterface` method.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
        enum Ev {
            Start,
            Stop,
            Next,
        }
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
        enum Out {
            BusyEnter,
            BusyExit,
            Step1Enter,
            Step1Exit,
            Step2Enter,
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Default)]
        enum Work {
            #[default]
            Step1,
            Step2,
        }
        struct WorkSpec;
        impl StateMachineSpec for WorkSpec {
            type State = Work;
            type InputEvent = Ev;
            type InputData = ();
            type OutputEvent = Out;
            const EDGES: EdgeTable<Work, Ev, (), Out> = &[(
                Work::Step1,
                &[(Ev::Next, &[(None, None, Some(Work::Step2))])],
            )];
            fn on_enter(s: Work) -> Option<Out> {
                match s {
                    Work::Step1 => Some(Out::Step1Enter),
                    Work::Step2 => Some(Out::Step2Enter),
                }
            }
            fn on_exit(s: Work) -> Option<Out> {
                match s {
                    Work::Step1 => Some(Out::Step1Exit),
                    Work::Step2 => None,
                }
            }
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Default)]
        enum Top {
            #[default]
            Idle,
            Busy,
        }
        struct TopSpec;
        impl StateMachineSpec for TopSpec {
            type State = Top;
            type InputEvent = Ev;
            type InputData = ();
            type OutputEvent = Out;
            const EDGES: EdgeTable<Top, Ev, (), Out> = &[
                (Top::Idle, &[(Ev::Start, &[(None, None, Some(Top::Busy))])]),
                (Top::Busy, &[(Ev::Stop, &[(None, None, Some(Top::Idle))])]),
            ];
            fn on_enter(s: Top) -> Option<Out> {
                match s {
                    Top::Busy => Some(Out::BusyEnter),
                    Top::Idle => None,
                }
            }
            fn on_exit(s: Top) -> Option<Out> {
                match s {
                    Top::Busy => Some(Out::BusyExit),
                    Top::Idle => None,
                }
            }
        }

        type WorkNode = LeafNode<WorkSpec, Ev, (), Out>;
        children! {
            enum TopChildren {
                Busy => WorkNode,
            }
        }

        let node = Node::<TopSpec, TopChildren>::new(enum_map! {
            Top::Busy => TopChildren::Busy(Node::new_leaf()),
            _ => TopChildren::None,
        });

        let mut sink = RecordingSink::new();
        let mut sm = StateMachineRoot::create(node, &mut sink);
        assert_eq!(sm.node.machine.current, Top::Idle); // boot stops at the leaf
        sink.clear();

        // Start: enter Busy, then its child machine's default cascades to Step1.
        sm.step(Ev::Start, &(), &mut sink);
        assert_eq!(
            sink.events(),
            &[Some(Out::BusyEnter), Some(Out::Step1Enter)],
        );
        sink.clear();

        // Next: a deeper handler wins — only the child machine transitions.
        sm.step(Ev::Next, &(), &mut sink);
        assert_eq!(
            sink.events(),
            &[Some(Out::Step1Exit), Some(Out::Step2Enter)]
        );
        sink.clear();

        // Stop: tear down bottom-up (child Step2 exit emits nothing here, then
        // Busy exits), then re-enter the Idle leaf (emits nothing).
        sm.step(Ev::Stop, &(), &mut sink);
        assert_eq!(sink.events(), &[Some(Out::BusyExit)]);
        assert_eq!(sm.node.machine.current, Top::Idle);
    }
}
