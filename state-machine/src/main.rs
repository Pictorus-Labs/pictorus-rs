//! Worked example demonstrating the codegen pattern for composite state
//! machines. Tree shape:
//!
//! ```text
//! Root (parallel)
//! ├── Foo  (states: F1, F2, F3)
//! │     └── on F2: Baz  (states: Z1, Z2, Z3)
//! └── Bar  (states: B1, B2, B3)
//! ```

use enum_map::{Enum, enum_map};
use state_machine::{LeafNode, MachineSnapshot, Node, NodeInterface, StateMachineSpec};

// ---- Foo spec ----------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Enum)]
pub enum FooStates {
    #[default]
    F1,
    F2,
    F3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FooTransitions {
    ToF2,
    ToF3,
    ToF1,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FooSpec;

impl FooSpec {
    const fn edge_lookup(s: FooStates, t: FooTransitions) -> Option<FooStates> {
        match s {
            FooStates::F1 => match t {
                FooTransitions::ToF2 => Some(FooStates::F2),
                FooTransitions::ToF3 => Some(FooStates::F3),
                _ => None,
            },
            FooStates::F2 => match t {
                FooTransitions::ToF3 => Some(FooStates::F3),
                FooTransitions::ToF1 => Some(FooStates::F1),
                _ => None,
            },
            FooStates::F3 => match t {
                FooTransitions::ToF1 => Some(FooStates::F1),
                _ => None,
            },
        }
    }
}

impl StateMachineSpec for FooSpec {
    type States = FooStates;
    type Transitions = FooTransitions;
    fn edge_lookup(s: Self::States, t: Self::Transitions) -> Option<Self::States> {
        Self::edge_lookup(s, t)
    }
}

// ---- Bar spec ----------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Enum)]
pub enum BarStates {
    #[default]
    B1,
    B2,
    B3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarTransitions {
    Advance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BarSpec;

impl BarSpec {
    const fn edge_lookup(s: BarStates, t: BarTransitions) -> Option<BarStates> {
        match (s, t) {
            (BarStates::B1, BarTransitions::Advance) => Some(BarStates::B2),
            (BarStates::B2, BarTransitions::Advance) => Some(BarStates::B3),
            (BarStates::B3, BarTransitions::Advance) => Some(BarStates::B1),
        }
    }
}

impl StateMachineSpec for BarSpec {
    type States = BarStates;
    type Transitions = BarTransitions;
    fn edge_lookup(s: Self::States, t: Self::Transitions) -> Option<Self::States> {
        Self::edge_lookup(s, t)
    }
}

// ---- Baz spec (child of Foo::F2) ---------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Enum)]
pub enum BazStates {
    #[default]
    Z1,
    Z2,
    Z3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BazTransitions {
    ToZ2,
    ToZ3,
    ToZ1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BazSpec;

impl BazSpec {
    const fn edge_lookup(s: BazStates, t: BazTransitions) -> Option<BazStates> {
        match s {
            BazStates::Z1 => match t {
                BazTransitions::ToZ2 => Some(BazStates::Z2),
                BazTransitions::ToZ3 => Some(BazStates::Z3),
                _ => None,
            },
            BazStates::Z2 => match t {
                BazTransitions::ToZ3 => Some(BazStates::Z3),
                BazTransitions::ToZ1 => Some(BazStates::Z1),
                _ => None,
            },
            BazStates::Z3 => match t {
                BazTransitions::ToZ1 => Some(BazStates::Z1),
                _ => None,
            },
        }
    }
}

impl StateMachineSpec for BazSpec {
    type States = BazStates;
    type Transitions = BazTransitions;
    fn edge_lookup(s: Self::States, t: Self::Transitions) -> Option<Self::States> {
        Self::edge_lookup(s, t)
    }
}

// ---- Flat input / output bundles ---------------------------------------

#[derive(Debug, Default, Clone, Copy)]
pub struct Input {
    pub foo: Option<FooTransitions>,
    pub bar: Option<BarTransitions>,
    pub baz: Option<BazTransitions>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Output {
    pub foo: MachineSnapshot<FooStates, FooTransitions>,
    pub bar: MachineSnapshot<BarStates, BarTransitions>,
    pub baz: MachineSnapshot<BazStates, BazTransitions>,
}

// ---- Leaf node: Bar (no children) --------------------------------------

pub type BarNode = LeafNode<BarSpec, Input, Output>;

pub fn build_bar_node() -> BarNode {
    LeafNode::new_leaf(
        |input: &Input| input.bar,
        |output: &mut Output, snapshot| output.bar = snapshot,
    )
}

// ---- Leaf node: Baz (no children, lives under Foo::F2) -----------------

pub type BazNode = LeafNode<BazSpec, Input, Output>;

pub fn build_baz_node() -> BazNode {
    LeafNode::new_leaf(
        |input: &Input| input.baz,
        |output: &mut Output, snapshot| output.baz = snapshot,
    )
}

// ---- Hierarchical node: Foo (F2 owns one parallel child: Baz) ----------

state_machine::children! {
    pub enum FooChildren {
        F2 => BazNode,
    }
}

pub type FooNode = Node<FooSpec, FooChildren>;

pub fn build_foo_node() -> FooNode {
    let children = enum_map! {
       FooStates::F1 => FooChildren::None,
       FooStates::F2 => FooChildren::F2(build_baz_node()),
       FooStates::F3 => FooChildren::None,
    };
    FooNode::new(
        children,
        |input: &Input| input.foo,
        |output: &mut Output, snapshot| output.foo = snapshot,
    )
}

// ---- Root: parallel container -----------------------------------------

pub type Root = (FooNode, BarNode);

pub fn build_root() -> Root {
    (build_foo_node(), build_bar_node())
}

fn main() {
    let mut root = build_root();
    let _ = root.step(&Input::default());
}

// ---- Tests -------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_snapshot_is_all_defaults() {
        let mut root = build_root();
        let out = root.step(&Input::default());
        assert_eq!(out, Output::default());
    }

    #[test]
    fn parallel_siblings_advance_independently() {
        let mut root = build_root();
        let out = root.step(&Input {
            foo: Some(FooTransitions::ToF3),
            bar: Some(BarTransitions::Advance),
            baz: None,
        });
        assert_eq!(out.foo.state, FooStates::F3);
        assert_eq!(out.foo.last_transition, Some(FooTransitions::ToF3));
        assert_eq!(out.bar.state, BarStates::B2);
        assert_eq!(out.bar.last_transition, Some(BarTransitions::Advance));
    }

    #[test]
    fn inactive_child_transition_is_silently_dropped() {
        // Foo starts at F1, Baz is inactive. Baz transition should not fire.
        let mut root = build_root();
        let out = root.step(&Input {
            baz: Some(BazTransitions::ToZ2),
            ..Default::default()
        });
        assert_eq!(out.baz.state, BazStates::Z1);
        assert_eq!(out.baz.last_transition, None);
    }

    #[test]
    fn child_does_not_consume_transition_on_activation_tick() {
        // Tick 1: enter F2 AND attempt a Baz transition in the same bundle.
        // Per design, Baz must remain at default this tick.
        let mut root = build_root();
        let out = root.step(&Input {
            foo: Some(FooTransitions::ToF2),
            baz: Some(BazTransitions::ToZ2),
            ..Default::default()
        });
        assert_eq!(out.foo.state, FooStates::F2);
        assert_eq!(out.baz.state, BazStates::Z1);
        assert_eq!(out.baz.last_transition, None);
    }

    #[test]
    fn child_transitions_on_tick_after_activation() {
        let mut root = build_root();
        // Tick 1: activate F2.
        root.step(&Input {
            foo: Some(FooTransitions::ToF2),
            ..Default::default()
        });
        // Tick 2: Baz is now active and can consume a transition.
        let out = root.step(&Input {
            baz: Some(BazTransitions::ToZ2),
            ..Default::default()
        });
        assert_eq!(out.baz.state, BazStates::Z2);
        assert_eq!(out.baz.last_transition, Some(BazTransitions::ToZ2));
    }

    #[test]
    fn exiting_parent_state_resets_child_immediately() {
        let mut root = build_root();
        root.step(&Input {
            foo: Some(FooTransitions::ToF2),
            ..Default::default()
        });
        let output = root.step(&Input {
            baz: Some(BazTransitions::ToZ3),
            ..Default::default()
        });
        // Sanity: Baz advanced.
        assert_eq!(output.baz.state, BazStates::Z3);

        // Exit F2 → Baz must reset within the same tick.
        let out = root.step(&Input {
            foo: Some(FooTransitions::ToF1),
            ..Default::default()
        });
        assert_eq!(out.foo.state, FooStates::F1);
        assert_eq!(out.baz.state, BazStates::Z1);
        assert_eq!(out.baz.last_transition, None);
    }

    #[test]
    fn invalid_transition_does_not_change_state() {
        let mut root = build_root();
        let out = root.step(&Input {
            foo: Some(FooTransitions::ToF1), // already at F1
            ..Default::default()
        });
        assert_eq!(out.foo.state, FooStates::F1);
        assert_eq!(out.foo.last_transition, None);
    }
}
