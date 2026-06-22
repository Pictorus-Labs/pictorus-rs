use crate::{Events, StateDiagramInterface, StateMachine};
use enum_map::EnumArray;
use pictorus_traits::{Pass, PassBy, ProcessBlock};

pub trait FloatSignalInput: StateDiagramInterface {
    type SourceSignal: Pass;
    fn input_from_float_signal(
        signals: PassBy<'_, Self::SourceSignal>,
    ) -> (Self::InputData, Self::InputEvent);
}

pub trait FloatSignalOutput: StateDiagramInterface
where
    Self::OutputEvent: EnumArray<u32> + Copy,
{
    type TargetSignal: Copy + Pass + Default;
    fn output_to_float_signal(output: &Events<Self::OutputEvent>) -> Self::TargetSignal;
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

pub struct StateMachineBlock<SD: StateDiagramInterface + FloatSignalInput + FloatSignalOutput>
where
    SD::OutputEvent: EnumArray<u32> + Copy,
{
    state_machine: StateMachine<SD>,
    events: Events<SD::OutputEvent>,
    buffer: SD::TargetSignal,
}

impl<SD: StateDiagramInterface + FloatSignalInput + FloatSignalOutput> StateMachineBlock<SD>
where
    SD::OutputEvent: EnumArray<u32> + Copy,
{
    pub fn new(state_diagram_interface: SD) -> Self {
        let mut events = Events::default();
        let state_machine = StateMachine::create(state_diagram_interface, &mut events);
        Self {
            state_machine,
            events,
            buffer: SD::TargetSignal::default(),
        }
    }
}

impl<SD: StateDiagramInterface + FloatSignalInput + FloatSignalOutput> Default
    for StateMachineBlock<SD>
where
    SD::OutputEvent: EnumArray<u32> + Copy,
{
    fn default() -> Self {
        const {
            panic!(
                "StateMachineBlock must be constructed with StateMachineBlock::new() to initialize the state machine and buffer properly, not Default::default()."
            )
        }
    }
}

impl<SD: StateDiagramInterface + FloatSignalInput + FloatSignalOutput> ProcessBlock
    for StateMachineBlock<SD>
where
    SD::OutputEvent: EnumArray<u32> + Copy,
{
    type Inputs = SD::SourceSignal;
    type Output = SD::TargetSignal;
    type Parameters = Parameter;

    fn process<'b>(
        &'b mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        let (input_data, input_event) = SD::input_from_float_signal(inputs);
        let mut event_sink = Events::default();
        self.state_machine
            .step(input_event, &input_data, &mut event_sink);

        self.events = event_sink;
        self.buffer = SD::output_to_float_signal(&self.events);
        self.buffer.as_by()
    }

    fn buffer<'b>(&'b self) -> PassBy<'b, Self::Output> {
        self.buffer.as_by()
    }
}
