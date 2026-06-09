//! # Three-level nesting — the shortest tour of a deep hierarchy
//!
//! Run with:  `cargo run --example nested_drone`
//!
//! Where `audio_player.rs` is wide (orthogonal regions, guards, priorities),
//! this example is *deep*. It is a single chain of three nested machines and
//! nothing else, so the only thing on display is what happens to the
//! enter / exit cascade as the tree gets taller.
//!
//! Topology (── = hierarchy):
//!
//! ```text
//! Flight (L1)
//! ├─ Grounded                         (leaf / simple state)
//! └─ Airborne  ── Nav (L2)            (composite)
//!     ├─ Hovering                     (leaf / simple state)
//!     └─ Cruising  ── Speed (L3)      (composite)
//!         ├─ Normal                   (leaf)
//!         └─ Boost                    (leaf)
//! ```
//!
//! So the deepest active configuration is three machines tall:
//! `Flight=Airborne / Nav=Cruising / Speed=Boost`.
//!
//! What to watch:
//!   * boot cascades default transitions DOWN only as far as the default path
//!     goes — here just to `Grounded`, since the L2/L3 subtrees are dormant.   (boot)
//!   * entering `Airborne` then `Cruising` activates the L2 and L3 subtrees,
//!     each running its own default transition as it comes alive.        (steps 1, 2)
//!   * a transition deep in the tree (`Boost`) touches only L3; the L1/L2
//!     ancestors just persist.                                                (step 3)
//!   * `Land` from the deepest state exits all three levels BOTTOM-UP in one
//!     step: Boost → Cruising → Airborne, then enters Grounded.               (step 4)

use enum_map::{Enum, enum_map};

use state_machine::*;

// ─── One shared event set for every level ──────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ev {
    Takeoff, // Grounded -> Airborne          (L1)
    Land,    // Airborne -> Grounded          (L1)
    Cruise,  // Hovering -> Cruising          (L2)
    Hover,   // Cruising -> Hovering          (L2)
    Boost,   // Normal   -> Boost             (L3)
    Slow,    // Boost    -> Normal            (L3)
}

// No guards in this example, so the input data carries nothing.
type Data = ();

// ─── Output events — one enter/exit pair per state, so the trace tells all ──
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
enum Out {
    GroundedEnter,
    GroundedExit,
    AirborneEnter,
    AirborneExit,
    HoveringEnter,
    HoveringExit,
    CruisingEnter,
    CruisingExit,
    NormalEnter,
    NormalExit,
    BoostEnter,
    BoostExit,
}

// ─── L3: Speed (a leaf machine, lives under Cruising) ──────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Default)]
enum Speed {
    #[default]
    Normal,
    Boost,
}

struct SpeedSpec;
impl StateMachineSpec for SpeedSpec {
    type State = Speed;
    type InputEvent = Ev;
    type InputData = Data;
    type OutputEvent = Out;

    const EDGES: EdgeTable<Speed, Ev, Data, Out> = &[
        (
            Speed::Normal,
            &[(Ev::Boost, &[(None, None, Some(Speed::Boost))])],
        ),
        (
            Speed::Boost,
            &[(Ev::Slow, &[(None, None, Some(Speed::Normal))])],
        ),
    ];

    fn on_enter(s: Speed) -> Option<Out> {
        match s {
            Speed::Normal => Some(Out::NormalEnter),
            Speed::Boost => Some(Out::BoostEnter),
        }
    }
    fn on_exit(s: Speed) -> Option<Out> {
        match s {
            Speed::Normal => Some(Out::NormalExit),
            Speed::Boost => Some(Out::BoostExit),
        }
    }
}

// ─── L2: Nav (composite: Cruising owns the L3 Speed machine) ───────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Default)]
enum Nav {
    #[default]
    Hovering,
    Cruising,
}

struct NavSpec;
impl StateMachineSpec for NavSpec {
    type State = Nav;
    type InputEvent = Ev;
    type InputData = Data;
    type OutputEvent = Out;

    const EDGES: EdgeTable<Nav, Ev, Data, Out> = &[
        (
            Nav::Hovering,
            &[(Ev::Cruise, &[(None, None, Some(Nav::Cruising))])],
        ),
        (
            Nav::Cruising,
            &[(Ev::Hover, &[(None, None, Some(Nav::Hovering))])],
        ),
    ];

    fn on_enter(s: Nav) -> Option<Out> {
        match s {
            Nav::Hovering => Some(Out::HoveringEnter),
            Nav::Cruising => Some(Out::CruisingEnter),
        }
    }
    fn on_exit(s: Nav) -> Option<Out> {
        match s {
            Nav::Hovering => Some(Out::HoveringExit),
            Nav::Cruising => Some(Out::CruisingExit),
        }
    }
}

// Nav's per-state children: Cruising nests the L3 Speed machine; Hovering (and
// any other leaf state) falls through to the generated `Leaf` variant. The
// `children!` macro writes the enum and the whole `NodeInterface` forwarding impl.
type SpeedNode = LeafNode<SpeedSpec, Ev, Data, Out>;
fn build_speed_node() -> SpeedNode {
    Node::new_leaf()
}

children! {
    enum NavChildren {
        Cruising => SpeedNode,
    }
}

type NavNode = Node<NavSpec, NavChildren>;

fn build_nav_node() -> NavNode {
    Node::new(enum_map! {
        Nav::Cruising => NavChildren::Cruising(build_speed_node()),
        _ => NavChildren::None,
    })
}

// ─── L1: Flight (composite: Airborne owns the L2 Nav machine) ──────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Default)]
enum Flight {
    #[default]
    Grounded,
    Airborne,
}

struct FlightSpec;
impl StateMachineSpec for FlightSpec {
    type State = Flight;
    type InputEvent = Ev;
    type InputData = Data;
    type OutputEvent = Out;

    const EDGES: EdgeTable<Flight, Ev, Data, Out> = &[
        (
            Flight::Grounded,
            &[(Ev::Takeoff, &[(None, None, Some(Flight::Airborne))])],
        ),
        (
            Flight::Airborne,
            &[(Ev::Land, &[(None, None, Some(Flight::Grounded))])],
        ),
    ];

    fn on_enter(s: Flight) -> Option<Out> {
        match s {
            Flight::Grounded => Some(Out::GroundedEnter),
            Flight::Airborne => Some(Out::AirborneEnter),
        }
    }
    fn on_exit(s: Flight) -> Option<Out> {
        match s {
            Flight::Grounded => Some(Out::GroundedExit),
            Flight::Airborne => Some(Out::AirborneExit),
        }
    }
}

// Flight's per-state children: Airborne nests the L2 Nav machine; Grounded
// falls through to the generated `Leaf` variant.
children! {
    enum FlightChildren {
        Airborne => NavNode,
    }
}

type FlightNode = Node<FlightSpec, FlightChildren>;

// ─── Assembling the three-level tree ───────────────────────────────────────
fn build_flight_node() -> FlightNode {
    Node::new(enum_map! {
    // Airborne nests Nav, whose Cruising in turn nests the L3 Speed leaf.
    Flight::Airborne => FlightChildren::Airborne(build_nav_node()),
    // Grounded has no nested machine.
    _ => FlightChildren::None,
    })
}

// ─── Inspecting the live configuration across all three levels ─────────────
fn config(sm: &StateMachineRoot<FlightNode>) -> String {
    match sm.root().state() {
        Flight::Grounded => "Grounded".to_string(),
        Flight::Airborne => match sm.root().active_child() {
            FlightChildren::Airborne(nav) => match nav.state() {
                Nav::Hovering => "Airborne / Hovering".to_string(),
                Nav::Cruising => match nav.active_child() {
                    NavChildren::Cruising(speed) => match speed.state() {
                        Speed::Normal => "Airborne / Cruising / Normal".to_string(),
                        Speed::Boost => "Airborne / Cruising / Boost".to_string(),
                    },
                    _ => unreachable!(),
                },
            },
            _ => unreachable!(),
        },
    }
}

fn show(label: &str, before: &str, emitted: &[Out], after: &str) {
    println!("\n▶ {label}");
    println!("    config: {before}  ->  {after}");
    println!("    emitted: {emitted:?}");
}

fn main() {
    // ── Boot ──────────────────────────────────────────────────────────────
    // The top default transition lands in Grounded, a leaf — so the cascade
    // stops there. The L2/L3 subtrees exist but are dormant, so nothing in
    // them is entered yet.
    let mut trace = Events::default();
    let mut sm = StateMachineRoot::create(build_flight_node(), &mut trace);
    println!("▶ boot (create)");
    println!("    config: <none>  ->  {}", config(&sm));
    println!("    emitted: {:?}", &trace.order);
    trace.clear();

    // ── Step 1: Takeoff — activate the L2 subtree ──────────────────────────
    // Grounded exits, Airborne enters, then Airborne's child (the Nav machine)
    // runs its OWN default transition into Hovering as it comes alive.
    let before = config(&sm);
    sm.step(Ev::Takeoff, &(), &mut trace);
    show(
        "step 1 — Takeoff: enter Airborne, then its Nav default cascades to Hovering",
        &before,
        &trace.order,
        &config(&sm),
    );
    trace.clear();

    // ── Step 2: Cruise — activate the L3 subtree ───────────────────────────
    // Inside Airborne, the Nav machine moves Hovering -> Cruising. Entering
    // Cruising activates the L3 Speed machine, which runs its default into
    // Normal. L1 (Airborne) just persists.
    let before = config(&sm);
    sm.step(Ev::Cruise, &(), &mut trace);
    show(
        "step 2 — Cruise: enter Cruising, then its Speed default cascades to Normal",
        &before,
        &trace.order,
        &config(&sm),
    );
    trace.clear();

    // ── Step 3: Boost — a transition at the very bottom ────────────────────
    // Only L3 moves (Normal -> Boost). The L1 and L2 ancestors are untouched:
    // a deep transition is local to its level.
    let before = config(&sm);
    sm.step(Ev::Boost, &(), &mut trace);
    show(
        "step 3 — Boost: only L3 (Speed) transitions; ancestors persist",
        &before,
        &trace.order,
        &config(&sm),
    );
    trace.clear();

    // ── Step 4: Land — exit all three levels in one step ───────────────────
    // The top-level Airborne -> Grounded edge fires. Before the source state
    // can leave, the whole active subtree beneath it is torn down BOTTOM-UP:
    // Boost (L3), then Cruising (L2), then Airborne (L1) exit in that order,
    // and finally Grounded is entered.
    let before = config(&sm);
    sm.step(Ev::Land, &(), &mut trace);
    show(
        "step 4 — Land: exit Boost -> Cruising -> Airborne (bottom-up), enter Grounded",
        &before,
        &trace.order,
        &config(&sm),
    );
    trace.clear();
}
