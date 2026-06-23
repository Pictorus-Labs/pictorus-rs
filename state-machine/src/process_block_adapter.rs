//! This module provides a struct and set of traits that allow state machines to be used as a [`pictorus_traits::ProcessBlock`] in a Pictorus diagram.
//! It is a bridge between the generic state machine abstractions and the Pictorus diagram abstractions.
//!
//! To use it you implement the [`FloatSignalConverter`] trait on a local marker type, declaring the
//! diagram it adapts and how to translate float signals to/from that diagram's input and output.
//! You can then use the [`StateMachineBlock`] struct to wrap your state machine and use it as a process
//! block in a Pictorus diagram.
//!
//! The conversion is anchored on a caller-defined marker type (rather than on the state diagram itself)
//! so that downstream crates can implement it without running into orphan-rule violations: the diagram
//! types (e.g. [`crate::StateDiagram`]) and this trait both live in this crate, so an impl keyed on the
//! diagram would be a foreign-trait-for-foreign-type impl. A local marker type sidesteps that.

use crate::{Events, StateDiagramInterface, StateMachine};
use enum_map::{EnumArray, EnumMap};
use pictorus_traits::{Pass, PassBy, ProcessBlock};

/// Convenient alias for the input-event type of a converter's diagram.
type DiagramInputEvent<C> =
    <<C as FloatSignalConverter>::Diagram as StateDiagramInterface>::InputEvent;
/// Convenient alias for the input-data type of a converter's diagram.
type DiagramInputData<C> =
    <<C as FloatSignalConverter>::Diagram as StateDiagramInterface>::InputData;
/// Convenient alias for the output-event type of a converter's diagram.
type DiagramOutputEvent<C> =
    <<C as FloatSignalConverter>::Diagram as StateDiagramInterface>::OutputEvent;

/// Translates between Pictorus float signals and a state diagram's inputs and outputs.
///
/// Implement this on a local marker type in your own crate. Because the implementing type is local,
/// the impl is allowed even though both [`FloatSignalConverter`] and the diagram type are defined here.
pub trait FloatSignalConverter
where
    DiagramInputEvent<Self>: EnumArray<bool>,
    DiagramOutputEvent<Self>: EnumArray<u32>,
{
    /// The state diagram interface this converter adapts.
    type Diagram: StateDiagramInterface;
    /// The float signal fed into the diagram as input.
    type SourceSignal: Pass;
    /// The float signal produced from the diagram's emitted events.
    type TargetSignal: Copy + Pass + Default;

    /// Convert an incoming float signal into the diagram's input data and per-event activation map.
    fn input_from_float_signal(
        signals: PassBy<'_, Self::SourceSignal>,
    ) -> (
        DiagramInputData<Self>,
        EnumMap<DiagramInputEvent<Self>, bool>,
    );

    /// Convert the diagram's emitted events into an outgoing float signal.
    fn output_to_float_signal(output: &Events<DiagramOutputEvent<Self>>) -> Self::TargetSignal;
}

pub struct Parameter {}

impl Default for Parameter {
    fn default() -> Self {
        Self::new()
    }
}

impl Parameter {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct StateMachineBlock<SD: StateDiagramInterface, C: FloatSignalConverter<Diagram = SD>>
where
    SD::OutputEvent: EnumArray<u32> + Copy,
    SD::InputEvent: EnumArray<bool> + Copy,
{
    state_machine: StateMachine<SD>,
    events: Events<SD::OutputEvent>,
    buffer: C::TargetSignal,
}

impl<SD: StateDiagramInterface, C: FloatSignalConverter<Diagram = SD>> StateMachineBlock<SD, C>
where
    SD::OutputEvent: EnumArray<u32> + Copy,
    SD::InputEvent: EnumArray<bool> + Copy,
{
    pub fn new(state_diagram_interface: SD) -> Self {
        let mut events = Events::default();
        let state_machine = StateMachine::create(state_diagram_interface, &mut events);
        let buffer = C::output_to_float_signal(&events);
        Self {
            state_machine,
            events,
            buffer,
        }
    }
}

impl<SD: StateDiagramInterface, C: FloatSignalConverter<Diagram = SD>> Default
    for StateMachineBlock<SD, C>
where
    SD::OutputEvent: EnumArray<u32> + Copy,
    SD::InputEvent: EnumArray<bool> + Copy,
{
    fn default() -> Self {
        const {
            panic!(
                "StateMachineBlock must be constructed with StateMachineBlock::new() to initialize the state machine and buffer properly, not Default::default()."
            )
        }
    }
}

impl<SD: StateDiagramInterface, C: FloatSignalConverter<Diagram = SD>> ProcessBlock
    for StateMachineBlock<SD, C>
where
    SD::OutputEvent: EnumArray<u32> + Copy,
    SD::InputEvent: EnumArray<bool> + Copy,
{
    type Inputs = C::SourceSignal;
    type Output = C::TargetSignal;
    type Parameters = Parameter;

    fn process<'b>(
        &'b mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        self.events.clear();
        let (input_data, input_event) = C::input_from_float_signal(inputs);
        self.state_machine
            .execute(input_event, &input_data, &mut self.events);
        self.buffer = C::output_to_float_signal(&self.events);
        self.buffer.as_by()
    }

    fn buffer<'b>(&'b self) -> PassBy<'b, Self::Output> {
        self.buffer.as_by()
    }
}

#[cfg(test)]
mod tests {
    use enum_map::Enum;

    use crate::{
        AllSimpleStateDiagram, EventTransitions, StateDiagramSpec, StateTransitions, Transition,
        TransitionTable,
    };
    use core::time::Duration;
    use pictorus_traits::{Context, PassBy};

    use super::*;

    #[derive(Copy, Clone, Enum, PartialEq, Default)]
    enum FooState {
        #[default]
        State1,
        State2,
    }

    #[derive(Copy, Clone, Enum, PartialEq)]
    enum InputEvent {
        Event1,
        Event2,
    }

    #[derive(Copy, Clone)]
    struct InputData {
        value: f64,
    }

    #[derive(Copy, Clone, Enum)]
    enum OutputEvent {
        EventA,
        EventB,
    }
    struct FooDiagramSpec;
    impl StateDiagramSpec for FooDiagramSpec {
        type InputData = InputData;
        type InputEvent = InputEvent;
        type OutputEvent = OutputEvent;
        type State = FooState;

        const TRANSITIONS: crate::TransitionTable<
            Self::State,
            Self::InputEvent,
            Self::InputData,
            Self::OutputEvent,
        > = TransitionTable::new(&[StateTransitions {
            source: FooState::State1,
            events: &[EventTransitions {
                event: InputEvent::Event1,
                transitions: &[Transition {
                    target: Some(FooState::State2),
                    action: Some(OutputEvent::EventA),
                    guard: Some(|input_data: &InputData| input_data.value > 0.0),
                }],
            }],
        }]);

        fn default_transition() -> (Self::State, Option<Self::OutputEvent>) {
            (FooState::State1, Some(OutputEvent::EventB))
        }

        fn during(_state: Self::State) -> Option<Self::OutputEvent> {
            None
        }

        fn on_enter(_state: Self::State) -> Option<Self::OutputEvent> {
            None
        }
        fn on_exit(_state: Self::State) -> Option<Self::OutputEvent> {
            None
        }
    }

    type FooDiagram = AllSimpleStateDiagram<FooDiagramSpec, InputEvent, InputData, OutputEvent>;
    fn build_foo_diagram() -> FooDiagram {
        FooDiagram::new_all_simple_states()
    }

    struct FooConverter;
    impl FloatSignalConverter for FooConverter {
        type Diagram = FooDiagram;
        type SourceSignal = [f64; 3];
        type TargetSignal = [f64; 2];

        fn input_from_float_signal(
            signals: PassBy<'_, Self::SourceSignal>,
        ) -> (
            DiagramInputData<Self>,
            EnumMap<DiagramInputEvent<Self>, bool>,
        ) {
            let signals = *signals;
            let input_data = InputData { value: signals[0] };
            let mut input_event = EnumMap::default();
            input_event[InputEvent::Event1] = signals[1] > 0.0;
            input_event[InputEvent::Event2] = signals[2] > 0.0;
            (input_data, input_event)
        }

        fn output_to_float_signal(output: &Events<DiagramOutputEvent<Self>>) -> Self::TargetSignal {
            let mut signals = [0.0; 2];
            if output.counts[OutputEvent::EventA] > 0 {
                signals[0] = 1.0;
            }
            if output.counts[OutputEvent::EventB] > 0 {
                signals[1] = 1.0;
            }
            signals
        }
    }

    #[derive(Debug, Copy, Clone)]
    pub struct StubContext {
        pub time: Duration,
        pub timestep: Option<Duration>,
        pub fundamental_timestep: Duration,
    }

    impl Default for StubContext {
        fn default() -> Self {
            Self {
                time: Duration::from_secs(0),
                timestep: None,
                fundamental_timestep: Duration::from_millis(100),
            }
        }
    }

    impl Context for StubContext {
        fn time(&self) -> Duration {
            self.time
        }

        fn timestep(&self) -> Option<Duration> {
            self.timestep
        }

        fn fundamental_timestep(&self) -> Duration {
            self.fundamental_timestep
        }
    }

    #[test]
    fn test_state_machine_block() {
        let foo_diagram = build_foo_diagram();
        let mut sm_block = StateMachineBlock::<_, FooConverter>::new(foo_diagram);
        let parameters = Parameter::new();
        let context = StubContext::default();
        assert_eq!(sm_block.buffer(), &[0.0, 1.0]); // default transition should emit EventB
        let input = [0.0, 1.0, 0.0];
        let output = sm_block.process(&parameters, &context, input.as_by());
        assert_eq!(output, &[0.0, 0.0]); // No events fire because the guard for Event1 is not satisfied
        let input = [42.0, 1.0, 0.0];
        let output = sm_block.process(&parameters, &context, input.as_by());
        assert_eq!(output, &[1.0, 0.0]); // EventA should be emitted due to Event1 being true
    }
}
