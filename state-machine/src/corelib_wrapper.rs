//! The core abstractions in this crate are somewhat generic. This module bridges the gap to make them usable
//! in pictorus diagrams.

use crate::NodeInterface;
use pictorus_traits::{Pass, PassBy, ProcessBlock};

pub trait FromFloatSignals {
    type SourceSignal: Pass;
    fn from_float_signals(signals: PassBy<'_, Self::SourceSignal>) -> Self;
}

pub trait ToFloatSignals {
    type TargetSignal: Pass + Default;
    fn to_float_signals(&self) -> Self::TargetSignal;
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

pub struct SmBlock<SM: NodeInterface, I: FromFloatSignals, O: ToFloatSignals> {
    state_machine: SM,
    buffer: O::TargetSignal,
    _input_marker: core::marker::PhantomData<I>,
    _output_marker: core::marker::PhantomData<O>,
}
impl<SM: NodeInterface, I: FromFloatSignals, O: ToFloatSignals> SmBlock<SM, I, O> {
    pub fn new(state_machine: SM) -> Self {
        Self {
            state_machine,
            buffer: O::TargetSignal::default(),
            _input_marker: core::marker::PhantomData,
            _output_marker: core::marker::PhantomData,
        }
    }
}

impl<SM: NodeInterface, I: FromFloatSignals, O: ToFloatSignals> Default for SmBlock<SM, I, O> {
    fn default() -> Self {
        const {
            panic!(
                "SmBlock must be constructed with SmBlock::new() to initialize the state machine and buffer properly, not Default::default()."
            )
        }
    }
}

impl<SM: NodeInterface, I: FromFloatSignals, O: ToFloatSignals> ProcessBlock for SmBlock<SM, I, O>
where
    SM: NodeInterface<Input = I, Output = O>,
{
    type Inputs = I::SourceSignal;
    type Output = O::TargetSignal;
    type Parameters = Parameter;

    fn process<'b>(
        &'b mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) -> pictorus_traits::PassBy<'b, Self::Output> {
        let input_struct = I::from_float_signals(inputs);
        let output_struct = self.state_machine.step(&input_struct);
        self.buffer = output_struct.to_float_signals();
        self.buffer.as_by()
    }

    fn buffer(&self) -> pictorus_traits::PassBy<'_, Self::Output> {
        self.buffer.as_by()
    }
}
