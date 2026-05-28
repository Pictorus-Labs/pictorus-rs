use enum_map::{Enum, enum_map};
use pictorus_traits::ProcessBlock;
use state_machine::{
    LeafNode, MachineSnapshot, Node, StateMachineSpec,
    corelib_wrapper::{FromFloatSignals, SmBlock, ToFloatSignals},
};
use strum::FromRepr;

// ---- Power spec --------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Enum)]
pub enum PowerStates {
    #[default]
    Off,
    On,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
pub enum PowerTransitions {
    Toggle,
}

pub struct PowerSpec;
impl PowerSpec {
    const fn edge_lookup(s: PowerStates, t: PowerTransitions) -> Option<PowerStates> {
        match (s, t) {
            (PowerStates::Off, PowerTransitions::Toggle) => Some(PowerStates::On),
            (PowerStates::On, PowerTransitions::Toggle) => Some(PowerStates::Off),
        }
    }
}
impl StateMachineSpec for PowerSpec {
    type States = PowerStates;
    type Transitions = PowerTransitions;
    fn edge_lookup(s: Self::States, t: Self::Transitions) -> Option<Self::States> {
        Self::edge_lookup(s, t)
    }
}

// ---- Display spec (child of Power::On) --------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Enum)]
pub enum DisplayStates {
    #[default]
    Standby,
    Active,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
pub enum DisplayTransitions {
    Wake,
    Sleep,
}

pub struct DisplaySpec;
impl DisplaySpec {
    const fn edge_lookup(s: DisplayStates, t: DisplayTransitions) -> Option<DisplayStates> {
        match (s, t) {
            (DisplayStates::Standby, DisplayTransitions::Wake) => Some(DisplayStates::Active),
            (DisplayStates::Active, DisplayTransitions::Sleep) => Some(DisplayStates::Standby),
            _ => None,
        }
    }
}
impl StateMachineSpec for DisplaySpec {
    type States = DisplayStates;
    type Transitions = DisplayTransitions;
    fn edge_lookup(s: Self::States, t: Self::Transitions) -> Option<Self::States> {
        Self::edge_lookup(s, t)
    }
}

// ---- Audio spec (child of Power::On) ----------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Enum)]
pub enum AudioStates {
    #[default]
    Mute,
    Playing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
pub enum AudioTransitions {
    Play,
    Pause,
}

pub struct AudioSpec;
impl AudioSpec {
    const fn edge_lookup(s: AudioStates, t: AudioTransitions) -> Option<AudioStates> {
        match (s, t) {
            (AudioStates::Mute, AudioTransitions::Play) => Some(AudioStates::Playing),
            (AudioStates::Playing, AudioTransitions::Pause) => Some(AudioStates::Mute),
            _ => None,
        }
    }
}
impl StateMachineSpec for AudioSpec {
    type States = AudioStates;
    type Transitions = AudioTransitions;
    fn edge_lookup(s: Self::States, t: Self::Transitions) -> Option<Self::States> {
        Self::edge_lookup(s, t)
    }
}

// ---- Flat input / output bundles --------------------------------------

#[derive(Debug, Default, Clone, Copy)]
pub struct Input {
    pub power: Option<PowerTransitions>,
    pub display: Option<DisplayTransitions>,
    pub audio: Option<AudioTransitions>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Output {
    pub power: MachineSnapshot<PowerStates, PowerTransitions>,
    pub display: MachineSnapshot<DisplayStates, DisplayTransitions>,
    pub audio: MachineSnapshot<AudioStates, AudioTransitions>,
}

// ---- Leaves ------------------------------------------------------------

pub type DisplayNode = LeafNode<DisplaySpec, Input, Output>;

pub fn build_display_node() -> DisplayNode {
    LeafNode::new_leaf(
        |input: &Input| input.display,
        |output: &mut Output, snapshot| output.display = snapshot,
    )
}

pub type AudioNode = LeafNode<AudioSpec, Input, Output>;

pub fn build_audio_node() -> AudioNode {
    LeafNode::new_leaf(
        |input: &Input| input.audio,
        |output: &mut Output, snapshot| output.audio = snapshot,
    )
}

// ---- Hierarchical Power with a paired child on the `On` state ---------
//
// The child of `On` is the tuple `(DisplayNode, AudioNode)`. The library
// provides `impl NodeInterface for (N1, N2)`, so the pair is itself a valid
// child node — no extra wiring required.

state_machine::children! {
    pub enum PowerChildren {
        On => (DisplayNode, AudioNode),
    }
}

pub type PowerNode = Node<PowerSpec, PowerChildren>;

pub fn build_power_node() -> PowerNode {
    let children = enum_map! {
        PowerStates::Off => PowerChildren::None,
        PowerStates::On => PowerChildren::On((build_display_node(), build_audio_node())),
    };
    PowerNode::new(
        children,
        |input: &Input| input.power,
        |output: &mut Output, snapshot| output.power = snapshot,
    )
}

// ---- Corelib wrapper --------------------------------------------------
impl ToFloatSignals for Output {
    type TargetSignal = [f64; 6];
    fn to_float_signals(&self) -> Self::TargetSignal {
        [
            self.power.state as u8 as f64,
            self.power.last_transition.map(|t| t as i16).unwrap_or(-1) as f64,
            self.display.state as u8 as f64,
            self.display.last_transition.map(|t| t as i16).unwrap_or(-1) as f64,
            self.audio.state as u8 as f64,
            self.audio.last_transition.map(|t| t as i16).unwrap_or(-1) as f64,
        ]
    }
}

impl FromFloatSignals for Input {
    type SourceSignal = [f64; 3];
    fn from_float_signals(signals: pictorus_traits::PassBy<'_, Self::SourceSignal>) -> Self {
        // Was going to use `from_usize` from the `enum_map::Enum` trait but it panics on out-of-range values
        Self {
            power: if signals[0] >= 0.0 {
                PowerTransitions::from_repr(signals[0] as usize)
            } else {
                None
            },
            display: if signals[1] >= 0.0 {
                DisplayTransitions::from_repr(signals[1] as usize)
            } else {
                None
            },
            audio: if signals[2] >= 0.0 {
                AudioTransitions::from_repr(signals[2] as usize)
            } else {
                None
            },
        }
    }
}

#[derive(Default)]
pub struct StubContext;
impl pictorus_traits::Context for StubContext {
    fn fundamental_timestep(&self) -> core::time::Duration {
        core::time::Duration::from_millis(10)
    }

    fn time(&self) -> core::time::Duration {
        core::time::Duration::from_millis(100)
    }

    fn timestep(&self) -> Option<core::time::Duration> {
        Some(core::time::Duration::from_millis(10))
    }
}

pub fn main() {
    let mut block: SmBlock<PowerNode, Input, Output> = SmBlock::new(build_power_node());
    let params = state_machine::corelib_wrapper::Parameter::new();
    let context = StubContext::default();

    // let output_vals = block.process(&params, &context, &[0.0, 0.0, 0.0]);

    //Miror flow of parallel_children.rs example but with the float shenanigans
    // Tick 1: turn power on. Per the activation rule, the children do not consume transitions this tick — they enter at their default.
    let out = block.process(
        &params,
        &context,
        &[
            PowerTransitions::Toggle as usize as f64,
            DisplayTransitions::Wake as usize as f64,
            AudioTransitions::Play as usize as f64,
        ],
    );
    assert_eq!(
        out,
        &[
            PowerStates::On as usize as f64,
            PowerTransitions::Toggle as i16 as f64,
            DisplayStates::Standby as usize as f64,
            -1.0, // No transition for display since it's inactive this tick
            AudioStates::Mute as usize as f64,
            -1.0, // No transition for audio since it's inactive this tick
        ]
    );

    // Tick 2: both children are active and advance in parallel from one bundle.
    let out = block.process(
        &params,
        &context,
        &[
            -1.0, // No power transition
            DisplayTransitions::Wake as usize as f64,
            AudioTransitions::Play as usize as f64,
        ],
    );
    assert_eq!(
        out,
        &[
            PowerStates::On as usize as f64,
            -1.0, // No power transition
            DisplayStates::Active as usize as f64,
            DisplayTransitions::Wake as i16 as f64,
            AudioStates::Playing as usize as f64,
            AudioTransitions::Play as i16 as f64,
        ]
    );

    // Tick 3: power off. Both children reset to default within the same tick.
    let out = block.process(
        &params,
        &context,
        &[
            PowerTransitions::Toggle as usize as f64,
            -1.0, // No display transition
            -1.0, // No audio transition
        ],
    );
    assert_eq!(
        out,
        &[
            PowerStates::Off as usize as f64,
            PowerTransitions::Toggle as i16 as f64,
            DisplayStates::Standby as usize as f64,
            -1.0, // No transition for display since it resets to default on the same tick
            AudioStates::Mute as usize as f64,
            -1.0, // No transition for audio since it resets to default on the same tick
        ]
    );
}
