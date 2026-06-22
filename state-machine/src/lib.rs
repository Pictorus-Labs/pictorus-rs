#![no_std]

#[cfg(any(feature = "std", test))]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod corelib_wrapper;

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

#[derive(Copy, Clone)]
pub struct UnguardedTransition<S: Copy, A: Copy> {
    pub action: Option<A>,
    pub target: Option<S>,
}

impl<S: Copy, D: 'static, A: Copy> From<&Transition<S, D, A>> for UnguardedTransition<S, A> {
    fn from(t: &Transition<S, D, A>) -> Self {
        Self {
            action: t.action,
            target: t.target,
        }
    }
}

/// A Guard is a function that takes the machine's InputData and returns a boolean indicating whether a transition is allowed to occur
pub type Guard<D> = fn(&D) -> bool;

/// A composite type representing a transition, consisting of an optional guard, an optional output event, and an optional target state
/// where None indicates an "internal transition"
pub struct Transition<S: Copy, D, A: Copy> {
    pub guard: Option<Guard<D>>,
    pub action: Option<A>,
    pub target: Option<S>,
}
/// A collection of transitions for a specific event, consisting of the event and a slice of transitions, ordered by priority
pub struct EventTransitions<S: 'static + Copy, E: 'static, D: 'static, A: 'static + Copy> {
    pub event: E,
    pub transitions: &'static [Transition<S, D, A>],
}

impl<S: 'static + Copy, E: 'static + PartialEq, D: 'static, A: 'static + Copy>
    EventTransitions<S, E, D, A>
{
    pub fn iter(&self) -> core::slice::Iter<'_, Transition<S, D, A>> {
        self.transitions.iter()
    }

    pub fn get_first_valid_transition(&self, data: &D) -> Option<UnguardedTransition<S, A>> {
        self.iter()
            .find(|t| t.guard.is_none_or(|g| g(data)))
            .map(UnguardedTransition::from)
    }
}
/// A collection of transitions from a specific state, consisting of the state and a slice of event transitions
pub struct StateTransitions<S: 'static + Copy, E: 'static, D: 'static, A: 'static + Copy> {
    pub source: S,
    pub events: &'static [EventTransitions<S, E, D, A>],
}

impl<S: 'static + Copy, E: 'static + PartialEq, D: 'static, A: 'static + Copy>
    StateTransitions<S, E, D, A>
{
    pub fn iter(&self) -> core::slice::Iter<'_, EventTransitions<S, E, D, A>> {
        self.events.iter()
    }
    pub fn find_event_transitions(&self, event: E) -> Option<&EventTransitions<S, E, D, A>> {
        self.iter().find(|e| e.event == event)
    }
}
/// The entire set of transitions for a state machine, represented as a slice of [`StateTransitions`]
pub struct TransitionTable<S: 'static + Copy, E: 'static, D: 'static, A: 'static + Copy>(
    &'static [StateTransitions<S, E, D, A>],
);
impl<S: 'static + PartialEq + Copy, E: 'static + PartialEq, D: 'static, A: 'static + Copy>
    TransitionTable<S, E, D, A>
{
    pub const fn empty() -> Self {
        Self(&[])
    }

    pub const fn new(transitions: &'static [StateTransitions<S, E, D, A>]) -> Self {
        Self(transitions)
    }

    pub fn iter(&self) -> core::slice::Iter<'_, StateTransitions<S, E, D, A>> {
        self.0.iter()
    }
    pub fn find_source_events(&self, source: S) -> Option<&StateTransitions<S, E, D, A>> {
        self.iter().find(|s| s.source == source)
    }

    pub fn resolve_transition(
        &self,
        source: S,
        event: E,
        data: &D,
    ) -> Option<UnguardedTransition<S, A>> {
        self.find_source_events(source)
            .and_then(|s| s.find_event_transitions(event))
            .and_then(|e| e.get_first_valid_transition(data))
    }
}

/// Core definition for a State Diagram's structure and behavior
/// This represents a single state diagram which can later be composed into a tree structure via
/// the [`StateDiagram`] struct and the [`StateDiagramInterface`] trait to form parallel regions and hierarchical states.
pub trait StateDiagramSpec {
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
    /// The default implementation of [`resolve`] looks up transitions in this table. Custom behavior
    /// could be implemented by leaving this empty (setting it to `&[]`) and overriding [`resolve`] with a custom function
    const TRANSITIONS: TransitionTable<
        Self::State,
        Self::InputEvent,
        Self::InputData,
        Self::OutputEvent,
    > = TransitionTable::empty();

    /// Given a source state, an event, and the current input data, resolve to the first valid outgoing transition (if any) according
    /// to the transition rules: find the matching state slice, then the matching event slice, then return the first transition whose
    ///  guard passes (or has no guard). The default implementation looks up transitions in the [`TRANSITIONS`] table. Custom behavior can be
    /// implemented by overriding this function.
    fn resolve(
        state: Self::State,
        event: Self::InputEvent,
        data: &Self::InputData,
    ) -> Option<UnguardedTransition<Self::State, Self::OutputEvent>> {
        Self::TRANSITIONS.resolve_transition(state, event, data)
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

/// Defines the interface for a composable element of a hierarchical/parallel state machine tree
pub trait StateDiagramInterface {
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

impl<A, B> StateDiagramInterface for (A, B)
where
    A: StateDiagramInterface,
    B: StateDiagramInterface<
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

impl<A, B, C> StateDiagramInterface for (A, B, C)
where
    A: StateDiagramInterface,
    B: StateDiagramInterface<
            InputEvent = A::InputEvent,
            InputData = A::InputData,
            OutputEvent = A::OutputEvent,
        >,
    C: StateDiagramInterface<
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

/// A struct that implements [`StateDiagramInterface`]
///
/// It stores the current active state and possibly a pending transition for this diagram.
/// For every state is stores a subtree of that state's children (in the case that none of the states have children, they will all be the placeholder type `NoChildren`).
pub struct StateDiagram<SMS: StateDiagramSpec, C>
where
    SMS::State: EnumArray<C>,
    C: StateDiagramInterface<
            OutputEvent = SMS::OutputEvent,
            InputEvent = SMS::InputEvent,
            InputData = SMS::InputData,
        >,
{
    current: SMS::State,
    pending: Option<UnguardedTransition<SMS::State, SMS::OutputEvent>>,
    children: EnumMap<SMS::State, C>,
}

impl<SMS: StateDiagramSpec, C> StateDiagram<SMS, C>
where
    SMS::State: EnumArray<C>,
    C: StateDiagramInterface<
            OutputEvent = SMS::OutputEvent,
            InputEvent = SMS::InputEvent,
            InputData = SMS::InputData,
        >,
{
    /// Create a new [`StateDiagram`] with the given children. The machine will be initialized to its default state.
    pub fn new(children: EnumMap<SMS::State, C>) -> Self {
        Self {
            current: SMS::State::default(),
            pending: None,
            children,
        }
    }
    /// The active state of this diagram.
    pub fn state(&self) -> SMS::State {
        self.current
    }

    /// The child subtree under the currently active state. Lets a caller that
    /// knows `C` (e.g. the machine's author) walk into the active branch.
    pub fn active_child(&self) -> &C {
        &self.children[self.current]
    }

    /// The child subtree under an arbitrary state.
    pub fn child(&self, state: SMS::State) -> &C {
        &self.children[state]
    }
}

impl<SMS: StateDiagramSpec>
    StateDiagram<SMS, NoChildren<SMS::InputEvent, SMS::InputData, SMS::OutputEvent>>
where
    SMS::State: EnumArray<NoChildren<SMS::InputEvent, SMS::InputData, SMS::OutputEvent>>,
{
    /// Create a new [`StateDiagram`] where none of the states have children
    pub fn new_all_simple_states() -> Self {
        Self {
            current: SMS::State::default(),
            pending: None,
            children: enum_map! { _ => NoChildren::default() },
        }
    }
}

impl<SMS: StateDiagramSpec, C> StateDiagramInterface for StateDiagram<SMS, C>
where
    SMS::State: EnumArray<C>,
    C: StateDiagramInterface<
            OutputEvent = SMS::OutputEvent,
            InputEvent = SMS::InputEvent,
            InputData = SMS::InputData,
        >,
{
    type InputEvent = SMS::InputEvent;
    type InputData = SMS::InputData;
    type OutputEvent = SMS::OutputEvent;

    fn select(&mut self, event: Self::InputEvent, data: &Self::InputData) -> bool {
        let s = self.current;

        // Child-first: a deeper handler preempts this level entirely.
        if self.children[s].select(event, data) {
            return true;
        }

        // Nobody below handled it — try our own outgoing edges.
        if let Some(edge) = SMS::resolve(s, event, data) {
            self.pending = Some(edge);
            return true;
        }

        false
    }

    fn execute_pending<K: EventSink<Self::OutputEvent>>(&mut self, sink: &mut K) {
        let pending = self.pending.take();
        // May be None if pending was none or if it is an internal transition
        let target_state = pending.as_ref().and_then(|e| e.target);
        // May be None if pending was none or if the edge has no action
        let transition_action = pending.as_ref().and_then(|e| e.action);

        if let Some(target_state) = target_state {
            // Need to do a full exit and enter sequence
            let old = self.current;

            self.children[old].exit(sink); // deeper exit actions first
            sink.emit_opt(SMS::on_exit(old)); // then this state's own
            self.children[old].reset(); // clean for next activation

            sink.emit_opt(transition_action); // transition action

            self.current = target_state;
            sink.emit_opt(SMS::on_enter(target_state));
            self.children[target_state].enter(sink); // cascade defaults ↓
        } else {
            // Internal transition or No transition, the only difference is that `transition_action`
            // may have been set if it is an internal transition, emit_opt will handle the None case correctly.
            let s = self.current;
            sink.emit_opt(SMS::during(s));
            sink.emit_opt(transition_action);
            self.children[s].execute_pending(sink);
        }
    }

    fn enter<K: EventSink<Self::OutputEvent>>(&mut self, sink: &mut K) {
        // Activating from default transition
        let (default_state, default_action) = SMS::default_transition();
        sink.emit_opt(default_action);
        self.current = default_state;
        sink.emit_opt(SMS::on_enter(default_state));
        self.children[default_state].enter(sink);
    }

    fn exit<K: EventSink<Self::OutputEvent>>(&mut self, sink: &mut K) {
        let s = self.current;
        self.children[s].exit(sink); // bottom-up
        sink.emit_opt(SMS::on_exit(s));
    }

    fn reset(&mut self) {
        self.current = SMS::State::default();
        self.pending = None;
        for c in self.children.values_mut() {
            c.reset();
        }
    }
}

/// A type alias for a state, which is a `StateDiagram` with `NoChildren`. This is a common case and the alias provides a convenient shorthand.
pub type AllSimpleStateDiagram<SMS, IE, ID, O> = StateDiagram<SMS, NoChildren<IE, ID, O>>;

/// A struct that when used as the `C` generic param of a [`StateDiagram`] represents the absence of any children
pub struct NoChildren<IE: Copy, ID, Out>(core::marker::PhantomData<(IE, ID, Out)>);
impl<IE: Copy, ID, Out> Default for NoChildren<IE, ID, Out> {
    fn default() -> Self {
        Self(core::marker::PhantomData)
    }
}
impl<IE: Copy, ID, Out: Copy> StateDiagramInterface for NoChildren<IE, ID, Out> {
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

/// Defines an enum that is used by [`StateDiagramInterface`] for assigning children to its composite States.
///
/// Each state's children are held in an `EnumMap<State, C>`, so every state must use
/// the *same* type `C` to store its children. The macro makes it easy to define that type as an enum where each variant
/// corresponds to a state that has children, and the variant's data is the type of the children for that state.
/// States that have no children are not listed in the enum, and are instead represented by a `None` variant that
/// the macro generates automatically. To invoke the macro, list only the states that nest a child machine, each as
/// `Variant => ChildType`.
///
/// ```ignore
/// children! {
///     pub enum NavChildren {
///         Cruising => SpeedDiagram,   // Cruising nests the L3 Speed machine
///         Landing => (LandingDiagram, CameraDiagram), // Landing nests the L3 Landing machine and the L4 Camera machine in parallel
///     }                            // Hovering (and any other state) -> None
/// }
/// });
/// ```
///
/// At least one composite variant is required: its type supplies the shared
/// `InputEvent` / `InputData` / `OutputEvent` for the enum.
/// If all states are simple (i.e. have no children), the `new_all_simple_states` constructor can be used instead of invoking the macro.
#[macro_export]
macro_rules! children {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $first_variant:ident => $first_child:ty
            $(, $variant:ident => $child:ty )*
            $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis enum $name {
            None,
            $first_variant($first_child),
            $( $variant($child), )*
        }

        impl $crate::StateDiagramInterface for $name {
            type InputEvent = <$first_child as $crate::StateDiagramInterface>::InputEvent;
            type InputData = <$first_child as $crate::StateDiagramInterface>::InputData;
            type OutputEvent = <$first_child as $crate::StateDiagramInterface>::OutputEvent;

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

/// The primary runtime interface for users of this library: a [`StateMachine`] is a wrapper around the top-level
/// [`StateDiagramInterface`] that provides a simple API for stepping the machine with input events and data, and for creating the machine.
pub struct StateMachine<S: StateDiagramInterface> {
    state_diagram: S,
}

impl<S: StateDiagramInterface> StateMachine<S> {
    pub fn create(state_diagram: S, sink: &mut impl EventSink<S::OutputEvent>) -> Self {
        let mut root = Self { state_diagram };
        root.state_diagram.enter(sink);
        root
    }

    pub fn step(
        &mut self,
        input_event: S::InputEvent,
        input_data: &S::InputData,
        sink: &mut impl EventSink<S::OutputEvent>,
    ) {
        self.state_diagram.select(input_event, input_data);
        self.state_diagram.execute_pending(sink);
    }
    /// Read-only access to the root state_diagram, for inspection.
    pub fn root(&self) -> &S {
        &self.state_diagram
    }
}
impl<S: StateDiagramInterface> StateMachine<S>
where
    S::InputEvent: EnumArray<bool>,
{
    pub fn execute(
        &mut self,
        input_events: EnumMap<S::InputEvent, bool>,
        input_data: &S::InputData,
        sink: &mut impl EventSink<S::OutputEvent>,
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
    impl StateDiagramSpec for TopSpec {
        type State = Top;
        type InputEvent = Ev;
        type InputData = Data;
        type OutputEvent = Out;

        const TRANSITIONS: TransitionTable<Top, Ev, Data, Out> = TransitionTable::new(&[
            StateTransitions {
                source: Top::Active,
                events: &[
                    EventTransitions {
                        event: Ev::Sleep,
                        transitions: &[
                            Transition {
                                guard: Some(|d: &Data| d.battery_ok),
                                action: Some(Out::TEffect),
                                target: Some(Top::Standby),
                            },
                            // Could have more edges here with different guards and/or no guards, which would be tried in order after this one.
                        ],
                    },
                    // Could have more events here with their own slices of edges.
                ],
            },
            // Standby has no outgoing edges in this slice of the example. So it is simply left out
        ]);

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
    impl StateDiagramSpec for AudioSpec {
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
    impl StateDiagramSpec for ScreenSpec {
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
    impl StateDiagramSpec for PowerSpec {
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
    impl StateDiagramSpec for NetworkSpec {
        type State = Network;
        type InputEvent = Ev;
        type InputData = Data;
        type OutputEvent = Out;

        // A region-internal transition, used to show `during` still fires on an
        // ancestor whose descendant transitions in the same step.
        const TRANSITIONS: TransitionTable<Network, Ev, Data, Out> =
            TransitionTable::new(&[StateTransitions {
                source: Network::Listening,
                events: &[EventTransitions {
                    event: Ev::Tick,
                    transitions: &[Transition {
                        guard: None,
                        action: Some(Out::NetworkConnect),
                        target: Some(Network::Connected),
                    }],
                }],
            }]);

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

    enum TopChildren {
        ActiveKids(
            (
                AllSimpleStateDiagram<AudioSpec, Ev, Data, Out>,
                AllSimpleStateDiagram<ScreenSpec, Ev, Data, Out>,
            ),
        ), // Audio ∥ Screen
        StandbyKids(
            (
                AllSimpleStateDiagram<PowerSpec, Ev, Data, Out>,
                AllSimpleStateDiagram<NetworkSpec, Ev, Data, Out>,
            ),
        ), // Power ∥ Network
    }

    impl StateDiagramInterface for TopChildren {
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

    type TopDiagram = StateDiagram<TopSpec, TopChildren>;

    fn build() -> TopDiagram {
        StateDiagram {
            current: Top::default(),
            pending: None,
            children: enum_map! {
                Top::Active  => TopChildren::ActiveKids((StateDiagram::new_all_simple_states(),  StateDiagram::new_all_simple_states())),
                Top::Standby => TopChildren::StandbyKids((StateDiagram::new_all_simple_states(), StateDiagram::new_all_simple_states())),
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
        let mut root_sm = StateMachine::create(root, &mut sink);
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
            Some(Out::ListeningEntry),   // ┘ …down to a simple state per region
        ];
        assert_eq!(sink.events(), expected);

        // Resulting stable configuration: <Standby – (LowPower, Listening)>.
        assert_eq!(root_sm.state_diagram.current, Top::Standby);
        match &root_sm.state_diagram.children[Top::Standby] {
            TopChildren::StandbyKids((power, network)) => {
                assert_eq!(power.current, Power::LowPower);
                assert_eq!(network.current, Network::Listening);
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
        let mut root_sm = StateMachine::create(build(), &mut sink);

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

        match &root_sm.state_diagram.children[Top::Standby] {
            TopChildren::StandbyKids((power, network)) => {
                assert_eq!(power.current, Power::LowPower); // persisted
                assert_eq!(network.current, Network::Connected); // moved
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
        impl StateDiagramSpec for Spec {
            type State = States;
            type InputEvent = Events;
            type InputData = Data;
            type OutputEvent = Out;

            // Tick → internal transition (None target), emitting InternalAct.
            const TRANSITIONS: TransitionTable<States, Events, Data, Out> =
                TransitionTable::new(&[StateTransitions {
                    source: States::Idle,
                    events: &[EventTransitions {
                        event: Events::Tick,
                        transitions: &[Transition {
                            guard: None,
                            action: Some(Out::InternalAct),
                            target: None, // internal transition
                        }],
                    }],
                }]);

            fn during(_: States) -> Option<Out> {
                Some(Out::IdleDuring)
            }
        }

        let mut sink = RecordingSink::new();
        let mut sm = StateMachine::create(
            StateDiagram::<Spec, NoChildren<Events, Data, Out>>::new_all_simple_states(),
            &mut sink,
        );
        sink.clear(); // create emits nothing here, but be explicit

        sm.step(Events::Tick, &Data { battery_ok: true }, &mut sink);

        // during BEFORE action; no on_enter/on_exit; state unchanged.
        assert_eq!(
            sink.events(),
            &[Some(Out::IdleDuring), Some(Out::InternalAct)],
        );
        assert_eq!(sm.state_diagram.current, States::Idle);
    }

    #[test]
    fn children_macro_forwards_and_cascades() {
        // A two-level machine assembled with `children!`: the parent `Busy`
        // state nests a child `Work` machine, while `Idle` has no children.
        // This exercises the generated enum and every forwarded `StateDiagramInterface` method.
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
        impl StateDiagramSpec for WorkSpec {
            type State = Work;
            type InputEvent = Ev;
            type InputData = ();
            type OutputEvent = Out;
            const TRANSITIONS: TransitionTable<Work, Ev, (), Out> =
                TransitionTable::new(&[StateTransitions {
                    source: Work::Step1,
                    events: &[EventTransitions {
                        event: Ev::Next,
                        transitions: &[Transition {
                            guard: None,
                            action: None,
                            target: Some(Work::Step2),
                        }],
                    }],
                }]);

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
        impl StateDiagramSpec for TopSpec {
            type State = Top;
            type InputEvent = Ev;
            type InputData = ();
            type OutputEvent = Out;
            const TRANSITIONS: TransitionTable<Top, Ev, (), Out> = TransitionTable::new(&[
                StateTransitions {
                    source: Top::Idle,
                    events: &[EventTransitions {
                        event: Ev::Start,
                        transitions: &[Transition {
                            guard: None,
                            action: None,
                            target: Some(Top::Busy),
                        }],
                    }],
                },
                StateTransitions {
                    source: Top::Busy,
                    events: &[EventTransitions {
                        event: Ev::Stop,
                        transitions: &[Transition {
                            guard: None,
                            action: None,
                            target: Some(Top::Idle),
                        }],
                    }],
                },
            ]);
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

        type WorkStateDiagram = AllSimpleStateDiagram<WorkSpec, Ev, (), Out>;
        children! {
            enum TopChildren {
                Busy => WorkStateDiagram,
            }
        }

        let diagram = StateDiagram::<TopSpec, TopChildren>::new(enum_map! {
            Top::Busy => TopChildren::Busy(StateDiagram::new_all_simple_states()),
            _ => TopChildren::None,
        });

        let mut sink = RecordingSink::new();
        let mut sm = StateMachine::create(diagram, &mut sink);
        assert_eq!(sm.state_diagram.current, Top::Idle); // boot stops at the top-level default, does not cascade into Busy
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
        // Busy exits), then re-enter the Idle State (emits nothing).
        sm.step(Ev::Stop, &(), &mut sink);
        assert_eq!(sink.events(), &[Some(Out::BusyExit)]);
        assert_eq!(sm.state_diagram.current, Top::Idle);
    }
}
