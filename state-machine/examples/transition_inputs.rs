//! Minimal example: a single parent state machine whose active state owns a
//! *pair* of sub-machines that run in parallel. The pair is just a tuple —
//! the library's tuple `NodeInterface` impl handles the parallel composition,
//! so no extra wiring is needed for "two children active at once."
//!
//! Tree shape:
//!
//! ```text
//! PowerNode (states: Off, On)
//! └── on On: (DisplayNode, AudioNode)   // both active simultaneously
//! ```

use enum_map::{Enum, enum_map};
use state_machine::{
    LeafNode, MachineSnapshot, Node, NodeInterface, StateMachineSpec, TransitionPrioritize,
};

// ---- Power spec --------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Enum)]
pub enum PowerStates {
    #[default]
    Off,
    On,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum AudioTransitions {
    Play,
    Pause,
}

pub struct AudioSpec;
impl AudioSpec {
    const fn edge_lookup(s: AudioStates, t: AudioTransitions) -> Option<AudioStates> {
        match (s, t) {
            (AudioStates::Mute, AudioTransitions::Play) => Some(AudioStates::Playing),
            (AudioStates::Mute, AudioTransitions::Pause) => Some(AudioStates::Mute),
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
    pub toggle: bool,
    pub play: bool,
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
        |input: &Input| {
            input
                .audio
                .prioritize_val(input.play, AudioTransitions::Play)
        },
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
        |input: &Input| {
            input
                .power
                .prioritize_val(input.toggle, PowerTransitions::Toggle)
        },
        |output: &mut Output, snapshot| output.power = snapshot,
    )
}

fn main() {
    let mut node = build_power_node();

    // Tick 1: turn power on. Per the activation rule, the children do not
    // consume transitions this tick — they enter at their default.
    let out = node.step(&Input {
        power: Some(PowerTransitions::Toggle),
        display: Some(DisplayTransitions::Wake),
        audio: Some(AudioTransitions::Play),
        ..Default::default()
    });
    assert_eq!(out.power.state, PowerStates::On);
    assert_eq!(out.display.state, DisplayStates::Standby);
    assert_eq!(out.audio.state, AudioStates::Mute);

    // Tick 2: both children are active and advance in parallel from one bundle.
    let out = node.step(&Input {
        display: Some(DisplayTransitions::Wake),
        audio: Some(AudioTransitions::Play),
        ..Default::default()
    });
    assert_eq!(out.power.state, PowerStates::On);
    assert_eq!(out.display.state, DisplayStates::Active);
    assert_eq!(out.audio.state, AudioStates::Playing);

    // Tick 3: power off. Both children reset to default within the same tick.
    let out = node.step(&Input {
        power: Some(PowerTransitions::Toggle),
        ..Default::default()
    });
    assert_eq!(out.power.state, PowerStates::Off);
    assert_eq!(out.display.state, DisplayStates::Standby);
    assert_eq!(out.display.last_transition, None);
    assert_eq!(out.audio.state, AudioStates::Mute);
    assert_eq!(out.audio.last_transition, None);

    // Tick 4: test toggle bool with input.power == None. Per the prioritize_val logic, the toggle should still work.
    let out = node.step(&Input {
        toggle: true,
        ..Default::default()
    });
    assert_eq!(out.power.state, PowerStates::On);

    // Tick 4 test that input.play is prioritized over the lower priority pause if it gets passed in on input.audio.
    let out = node.step(&Input {
        play: true,
        audio: Some(AudioTransitions::Pause),
        ..Default::default()
    });
    assert_eq!(out.audio.state, AudioStates::Playing); // Play wins over Pause due to prioritize_val logic
}
