//! # Handheld media player — a worked tour of the hierarchical state machine
//!
//! Run with:  `cargo run --example handheld_player`
//!
//! This example is deliberately over-instrumented: nearly every state has
//! entry / exit / during actions so that the *order* of emitted output events
//! tells the whole story of what the engine did on each step. Watch the printed
//! event stream against the commentary in `main`.
//!
//! Topology (── = hierarchy, ∥ = orthogonal regions):
//!
//! ```text
//! Player (top region)
//! ├─ Active   (composite)      Audio ∥ Screen
//! │   ├─ Audio  region:  Playing ⇄ Paused        (toggled by Button)
//! │   └─ Screen region:  Bright  ⇄ Dim           (toggled by Button)
//! ├─ Standby  (composite)      Power ∥ Net
//! │   ├─ Power  region:  LowPower
//! │   └─ Net    region:  Listening ⇄ Connected   (toggled by Button)
//! └─ Off      (simple / trap state)
//! ```
//!
//! Features exercised, with the step in `main` that shows each one:
//!   * default-transition cascade on creation                       (boot)
//!   * `during` fires while a state persists, top-down               (step 1)
//!   * internal transition: action fires, NO on_exit/on_enter, and
//!     `during` STILL fires — ordered *before* the internal action   (steps 1, 4)
//!   * priority resolution: 3 edges share `Tick`, first enabled wins (steps 1, 2)
//!   * guards reading InputData (same state, different data → different
//!     edge: see Standby Tick discarded vs. trickle)                 (steps 2, 3, 4)
//!   * external transition: bottom-up exit, action, top-down entry   (step 2)
//!   * event silently discarded — but `during` still runs            (step 3)
//!   * ancestor `during` fires even while a descendant transitions   (step 5)
//!   * default re-entry of a composite on transition-in              (step 6)
//!   * child-first preemption ("deeper handler wins") vs. fallthrough (steps 7, 9)
//!   * concurrent transitions in orthogonal regions                  (step 8)
//!   * `StateMachineRoot::execute` draining a multi-event queue in
//!     input-event priority order (enum order)                       (step 10)

use enum_map::{Enum, enum_map};

use state_machine::{
    EventTransitions, Events, StateDiagram, StateDiagramSpec, StateMachine, StateTransitions,
    Transition, TransitionTable, children,
};

// ─── Input events ────────────────────────────────────────────────────────
// The *enum order* is the input-event priority used by `StateMachineRoot::execute`
// when several events are queued in one timestep: Fault is highest, Tick lowest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
enum InputEvent {
    Fault,
    Sleep,
    Wake,
    Button,
    Tick,
}

// ─── Input data (immutable per step, visible to every guard) ──────────────
#[derive(Clone, Copy)]
struct InputData {
    battery: u8,    // 0..=100
    charging: bool, // plugged in?
}

// ─── Output events — named for the action they stand for ──────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
enum OutputEvent {
    // top / Player
    BootActive,
    ActiveEnter,
    ActiveExit,
    ActiveTick,  // during(Active)
    GoStandby,   // Active --Sleep--> Standby
    AutoSleep,   // Active --Tick[low batt]--> Standby
    Heartbeat,   // Active --Tick--> (internal)
    BatteryDead, // --Tick[batt==0]--> Off
    Crashed,     // --Fault--> Off
    StandbyEnter,
    StandbyExit,
    StandbyIdle, // during(Standby)
    Trickle,     // Standby --Tick[charging]--> (internal)
    Waking,      // Standby --Wake--> Active
    PoweredOff,  // on_enter(Off)
    // Audio region
    AudioInit,    // default-transition action
    AudioPlay,    // on_enter(Playing)
    AudioStop,    // on_exit(Playing)
    AudioPaused,  // on_enter(Paused)
    Pause,        // Playing --Button--> Paused
    Resume,       // Paused --Button--> Playing
    Decoding,     // during(Playing)
    AudioRecover, // Playing --Fault--> (internal): absorbs the fault
    // Screen region
    ScreenInit,
    ScreenBright,     // on_enter(Bright)
    ScreenBrightExit, // on_exit(Bright)
    ScreenDim,        // on_enter(Dim)
    DimScreen,        // Bright --Button--> Dim
    BrightScreen,     // Dim --Button--> Bright
    Rendering,        // during(Bright)
    // Power region
    PowerInit,
    PowerLow,  // on_enter(LowPower)
    PowerSave, // during(LowPower)
    // Net region
    NetInit,
    NetListen,     // on_enter(Listening)
    NetListenExit, // on_exit(Listening)
    NetConnected,  // on_enter(Connected)
    NetConnect,    // Listening --Button--> Connected
    NetDisconnect, // Connected --Button--> Listening
    Scanning,      // during(Listening)
}

// ─── Player Region ──────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Default)]
enum Player {
    #[default]
    Active,
    Standby,
    Off,
}

// Player State machine. The Tick row under `Active` is the priority showcase: three
// edges, all keyed on Tick, tried top-to-bottom. The first whose guard passes
// wins; the final unguarded edge is an *internal* transition (target = None)
// that acts as the catch-all "nothing special happened this tick".
struct PlayerSpec;
impl StateDiagramSpec for PlayerSpec {
    type State = Player;
    type InputEvent = InputEvent;
    type InputData = InputData;
    type OutputEvent = OutputEvent;

    const TRANSITIONS: TransitionTable<
        Self::State,
        Self::InputEvent,
        Self::InputData,
        Self::OutputEvent,
    > = TransitionTable::new(&[
        StateTransitions {
            source: Player::Active,
            events: &[
                EventTransitions {
                    event: InputEvent::Sleep,
                    transitions: &[Transition {
                        guard: Some(|c: &InputData| c.battery > 0),
                        action: Some(OutputEvent::GoStandby),
                        target: Some(Player::Standby),
                    }],
                },
                EventTransitions {
                    event: InputEvent::Fault,
                    transitions: &[Transition {
                        guard: None,
                        action: Some(OutputEvent::Crashed),
                        target: Some(Player::Off),
                    }],
                },
                EventTransitions {
                    event: InputEvent::Tick,
                    transitions: &[
                        Transition {
                            guard: Some(|c: &InputData| c.battery == 0),
                            action: Some(OutputEvent::BatteryDead),
                            target: Some(Player::Off),
                        },
                        Transition {
                            guard: Some(|c: &InputData| c.battery < 20 && !c.charging),
                            action: Some(OutputEvent::AutoSleep),
                            target: Some(Player::Standby),
                        },
                        Transition {
                            guard: None,
                            action: Some(OutputEvent::Heartbeat),
                            target: None, // internal transition — no state change
                        },
                    ],
                },
            ],
        },
        StateTransitions {
            source: Player::Standby,
            events: &[
                EventTransitions {
                    event: InputEvent::Wake,
                    transitions: &[Transition {
                        guard: None,
                        action: Some(OutputEvent::Waking),
                        target: Some(Player::Active),
                    }],
                },
                EventTransitions {
                    event: InputEvent::Fault,
                    transitions: &[Transition {
                        guard: None,
                        action: Some(OutputEvent::Crashed),
                        target: Some(Player::Off),
                    }],
                },
                EventTransitions {
                    event: InputEvent::Tick,
                    transitions: &[
                        Transition {
                            guard: Some(|c: &InputData| c.battery == 0),
                            action: Some(OutputEvent::BatteryDead),
                            target: Some(Player::Off),
                        },
                        Transition {
                            guard: Some(|c: &InputData| c.charging),
                            action: Some(OutputEvent::Trickle),
                            target: None, // internal transition
                        },
                    ],
                },
            ],
        },
    ]);

    fn on_enter(s: Player) -> Option<OutputEvent> {
        match s {
            Player::Active => Some(OutputEvent::ActiveEnter),
            Player::Standby => Some(OutputEvent::StandbyEnter),
            Player::Off => Some(OutputEvent::PoweredOff),
        }
    }
    fn on_exit(s: Player) -> Option<OutputEvent> {
        match s {
            Player::Active => Some(OutputEvent::ActiveExit),
            Player::Standby => Some(OutputEvent::StandbyExit),
            Player::Off => None,
        }
    }
    fn during(s: Player) -> Option<OutputEvent> {
        match s {
            Player::Active => Some(OutputEvent::ActiveTick),
            Player::Standby => Some(OutputEvent::StandbyIdle),
            Player::Off => None,
        }
    }
    fn default_transition() -> (Player, Option<OutputEvent>) {
        (Player::Active, Some(OutputEvent::BootActive))
    }
}

// ─── Audio region ──────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Default)]
enum Audio {
    #[default]
    Playing,
    Paused,
}

struct AudioSpec;
impl StateDiagramSpec for AudioSpec {
    type State = Audio;
    type InputEvent = InputEvent;
    type InputData = InputData;
    type OutputEvent = OutputEvent;

    const TRANSITIONS: TransitionTable<Audio, InputEvent, InputData, OutputEvent> =
        TransitionTable::new(&[
            StateTransitions {
                source: Audio::Playing,
                events: &[
                    EventTransitions {
                        event: InputEvent::Button,
                        transitions: &[Transition {
                            guard: None,
                            action: Some(OutputEvent::Pause),
                            target: Some(Audio::Paused),
                        }],
                    },
                    EventTransitions {
                        // Internal: while Playing, the Audio region ABSORBS a Fault
                        // (e.g. transient decoder hiccup) instead of letting it bubble
                        // up to the Active→Off edge. Because a deeper handler wins, the
                        // top-level Fault transition is never even evaluated.
                        event: InputEvent::Fault,
                        transitions: &[Transition {
                            guard: None,
                            action: Some(OutputEvent::AudioRecover),
                            target: None, // internal transition — absorbs the fault
                        }],
                    },
                ],
            },
            StateTransitions {
                // Paused has NO Fault edge — so a Fault while Paused is NOT absorbed
                // here and propagates up to the Active→Off transition.
                source: Audio::Paused,
                events: &[EventTransitions {
                    event: InputEvent::Button,
                    transitions: &[Transition {
                        guard: None,
                        action: Some(OutputEvent::Resume),
                        target: Some(Audio::Playing),
                    }],
                }],
            },
        ]);

    fn on_enter(s: Audio) -> Option<OutputEvent> {
        match s {
            Audio::Playing => Some(OutputEvent::AudioPlay),
            Audio::Paused => Some(OutputEvent::AudioPaused),
        }
    }
    fn on_exit(s: Audio) -> Option<OutputEvent> {
        match s {
            Audio::Playing => Some(OutputEvent::AudioStop),
            Audio::Paused => None,
        }
    }
    fn during(s: Audio) -> Option<OutputEvent> {
        match s {
            Audio::Playing => Some(OutputEvent::Decoding),
            Audio::Paused => None,
        }
    }
    fn default_transition() -> (Audio, Option<OutputEvent>) {
        (Audio::Playing, Some(OutputEvent::AudioInit))
    }
}

// ─── Screen region ──────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Default)]
enum Screen {
    #[default]
    Bright,
    Dim,
}
struct ScreenSpec;
impl StateDiagramSpec for ScreenSpec {
    type State = Screen;
    type InputEvent = InputEvent;
    type InputData = InputData;
    type OutputEvent = OutputEvent;

    const TRANSITIONS: TransitionTable<Screen, InputEvent, InputData, OutputEvent> =
        TransitionTable::new(&[
            StateTransitions {
                source: Screen::Bright,
                events: &[EventTransitions {
                    event: InputEvent::Button,
                    transitions: &[Transition {
                        guard: None,
                        action: Some(OutputEvent::DimScreen),
                        target: Some(Screen::Dim),
                    }],
                }],
            },
            StateTransitions {
                source: Screen::Dim,
                events: &[EventTransitions {
                    event: InputEvent::Button,
                    transitions: &[Transition {
                        guard: None,
                        action: Some(OutputEvent::BrightScreen),
                        target: Some(Screen::Bright),
                    }],
                }],
            },
        ]);

    fn on_enter(s: Screen) -> Option<OutputEvent> {
        match s {
            Screen::Bright => Some(OutputEvent::ScreenBright),
            Screen::Dim => Some(OutputEvent::ScreenDim),
        }
    }
    fn on_exit(s: Screen) -> Option<OutputEvent> {
        match s {
            Screen::Bright => Some(OutputEvent::ScreenBrightExit),
            Screen::Dim => None,
        }
    }
    fn during(s: Screen) -> Option<OutputEvent> {
        match s {
            Screen::Bright => Some(OutputEvent::Rendering),
            Screen::Dim => None,
        }
    }
    fn default_transition() -> (Screen, Option<OutputEvent>) {
        (Screen::Bright, Some(OutputEvent::ScreenInit))
    }
}

// ─── Power region ──────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Default)]
enum Power {
    #[default]
    LowPower,
}

struct PowerSpec;
impl StateDiagramSpec for PowerSpec {
    type State = Power;
    type InputEvent = InputEvent;
    type InputData = InputData;
    type OutputEvent = OutputEvent;
    fn on_enter(_: Power) -> Option<OutputEvent> {
        Some(OutputEvent::PowerLow)
    }
    fn during(_: Power) -> Option<OutputEvent> {
        Some(OutputEvent::PowerSave)
    }
    fn default_transition() -> (Power, Option<OutputEvent>) {
        (Power::LowPower, Some(OutputEvent::PowerInit))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Default)]
enum Net {
    #[default]
    Listening,
    Connected,
}

struct NetSpec;
impl StateDiagramSpec for NetSpec {
    type State = Net;
    type InputEvent = InputEvent;
    type InputData = InputData;
    type OutputEvent = OutputEvent;

    const TRANSITIONS: TransitionTable<Net, InputEvent, InputData, OutputEvent> =
        TransitionTable::new(&[
            StateTransitions {
                source: Net::Listening,
                events: &[EventTransitions {
                    event: InputEvent::Button,
                    transitions: &[Transition {
                        guard: None,
                        action: Some(OutputEvent::NetConnect),
                        target: Some(Net::Connected),
                    }],
                }],
            },
            StateTransitions {
                source: Net::Connected,
                events: &[EventTransitions {
                    event: InputEvent::Button,
                    transitions: &[Transition {
                        guard: None,
                        action: Some(OutputEvent::NetDisconnect),
                        target: Some(Net::Listening),
                    }],
                }],
            },
        ]);

    fn on_enter(s: Net) -> Option<OutputEvent> {
        match s {
            Net::Listening => Some(OutputEvent::NetListen),
            Net::Connected => Some(OutputEvent::NetConnected),
        }
    }
    fn on_exit(s: Net) -> Option<OutputEvent> {
        match s {
            Net::Listening => Some(OutputEvent::NetListenExit),
            Net::Connected => None,
        }
    }
    fn during(s: Net) -> Option<OutputEvent> {
        match s {
            Net::Listening => Some(OutputEvent::Scanning),
            Net::Connected => None,
        }
    }
    fn default_transition() -> (Net, Option<OutputEvent>) {
        (Net::Listening, Some(OutputEvent::NetInit))
    }
}

// ─── Assembling the tree ───────────────────────────────────────────────────
// Here we describe the topology of the state machine: which states are nested inside which, and which regions are orthogonal.

/// For convenience, a type alias for any state diagram with all simple states in this example that sets the type parameters which will be the same for
/// every state diagram that makes up the State Machine
type AllSimpleStateDiagram<S> =
    state_machine::AllSimpleStateDiagram<S, InputEvent, InputData, OutputEvent>;

type AudioNode = AllSimpleStateDiagram<AudioSpec>;
type ScreenNode = AllSimpleStateDiagram<ScreenSpec>;
type PowerNode = AllSimpleStateDiagram<PowerSpec>;
type NetNode = AllSimpleStateDiagram<NetSpec>;

children! {
    enum PlayerChildren {
    Active => (AudioNode, ScreenNode), // Audio ∥ Screen
    Standby => (PowerNode, NetNode),   // Power ∥ Net
}}

type PlayerNode = StateDiagram<PlayerSpec, PlayerChildren>;

fn build() -> PlayerNode {
    StateDiagram::new(enum_map! {
        Player::Active  => PlayerChildren::Active((AudioNode::new_all_simple_states(), ScreenNode::new_all_simple_states())),
        Player::Standby => PlayerChildren::Standby((PowerNode::new_all_simple_states(), NetNode::new_all_simple_states())),
        Player::Off     => PlayerChildren::None,
    })
}

// ─── Inspecting the live configuration (purely for the printout) ───────────
fn config(sm: &StateMachine<PlayerNode>) -> String {
    let top = sm.root().state();
    match (&top, &sm.root().active_child()) {
        (Player::Active, PlayerChildren::Active((a, s))) => {
            format!("Active( Audio={:?}, Screen={:?} )", a.state(), s.state())
        }
        (Player::Standby, PlayerChildren::Standby((p, n))) => {
            format!("Standby( Power={:?}, Net={:?} )", p.state(), n.state())
        }
        (Player::Off, _) => "Off".to_string(),
        _ => unreachable!("configuration and children variant always agree"),
    }
}

fn show(label: &str, before: &str, emitted: &[OutputEvent], after: &str) {
    println!("\n▶ {label}");
    println!("    config: {before}  ->  {after}");
    println!("    emitted: {emitted:?}");
}

fn main() {
    // ── Boot ────────────────────────────────────────────────────────────
    // `create` runs the top default transition, then cascades default
    // transitions down through both of Active's regions to stable leaves.
    let mut output_events = Events::default();
    let mut sm = StateMachine::create(build(), &mut output_events);
    println!("▶ boot (create)");
    println!("    config: <none>  ->  {}", config(&sm));
    println!("    emitted: {:?}", output_events.order);
    println!(
        "    (top default action, top on_enter, then each region's default \
         action + on_enter, top-down)"
    );
    output_events.clear();

    let healthy = InputData {
        battery: 80,
        charging: false,
    };
    let low = InputData {
        battery: 10,
        charging: false,
    };

    // ── Step 1: Tick, healthy battery ─────────────────────────────────────
    // No region handles Tick, so Active evaluates its own Tick edges. Guards
    // for edge 1 (batt==0) and edge 2 (batt<20) fail; the unguarded internal
    // edge wins. Internal => no on_exit/on_enter, but `during` fires at every
    // persisting level, top-down, and the internal action (Heartbeat) is
    // ordered immediately AFTER Active's own `during`.
    let before = config(&sm);
    sm.step(InputEvent::Tick, &healthy, &mut output_events);
    show(
        "step 1 — Tick @80%: priority falls through to the internal heartbeat",
        &before,
        &output_events.order,
        &config(&sm),
    );
    output_events.clear();

    // ── Step 2: Tick, critically low + unplugged → external transition ─────
    // Now edge 2's guard (batt<20 && !charging) passes and edge 2 fires: an
    // EXTERNAL transition Active→Standby. We're still in Playing/Bright, whose
    // states carry exit actions, so the shape is plain to see: exit is
    // bottom-up (AudioStop, ScreenBrightExit, then ActiveExit), then the
    // transition action, then entry top-down (Standby, then each region's
    // default cascade). No `during` anywhere — this step *is* an external
    // transition at the top.
    let before = config(&sm);
    sm.step(InputEvent::Tick, &low, &mut output_events);
    show(
        "step 2 — Tick @10% unplugged: external Active->Standby (exit↑, action, enter↓)",
        &before,
        &output_events.order,
        &config(&sm),
    );
    output_events.clear();

    // ── Step 3: Tick in Standby, not charging → discarded, but `during` runs ─
    // In Standby, Tick edge 1 (batt==0) fails and edge 2 (charging) fails, so
    // Tick matches NO edge and is "silently discarded" as a transition. The
    // engine still walks the active tree and fires `during` at every level.
    let before = config(&sm);
    sm.step(InputEvent::Tick, &low, &mut output_events);
    show(
        "step 3 — Tick in Standby, unplugged: no edge matches, yet `during` fires",
        &before,
        &output_events.order,
        &config(&sm),
    );
    output_events.clear();

    // ── Step 4: Tick in Standby while charging → internal trickle ──────────
    // Same state, different data: edge 2's guard (charging) now passes, firing
    // an internal transition. `during` still fires at each level; the internal
    // action (Trickle) is ordered right after Standby's own `during`.
    let charging = InputData {
        battery: 10,
        charging: true,
    };
    let before = config(&sm);
    sm.step(InputEvent::Tick, &charging, &mut output_events);
    show(
        "step 4 — Tick in Standby, charging: internal trickle (during + internal action)",
        &before,
        &output_events.order,
        &config(&sm),
    );
    output_events.clear();

    // ── Step 5: Button in Standby — region transitions, ancestor persists ──
    // Only the Net region has a Button edge (Listening→Connected). Standby and
    // the Power region persist, so their `during`s fire; Net does a real
    // external transition (on_exit Listening, action, on_enter Connected) and
    // therefore does NOT fire its own `during`.
    let before = config(&sm);
    sm.step(InputEvent::Button, &low, &mut output_events);
    show(
        "step 5 — Button in Standby: Net transitions while Standby/Power persist",
        &before,
        &output_events.order,
        &config(&sm),
    );
    output_events.clear();

    // ── Step 6: Wake → back to Active (fresh region defaults) ──────────────
    let before = config(&sm);
    sm.step(InputEvent::Wake, &healthy, &mut output_events);
    show(
        "step 6 — Wake: Standby->Active, regions re-enter via their defaults",
        &before,
        &output_events.order,
        &config(&sm),
    );
    output_events.clear();

    // ── Step 7: Fault while Audio is Playing — absorbed by the child ───────
    // Audio::Playing has a Fault edge (internal AudioRecover). Child-first
    // selection means a deeper handler wins, so the Active→Off Fault edge is
    // never evaluated: the fault is absorbed and we stay in Active.
    let before = config(&sm);
    sm.step(InputEvent::Fault, &healthy, &mut output_events);
    show(
        "step 7 — Fault while Playing: ABSORBED by Audio (deeper handler wins)",
        &before,
        &output_events.order,
        &config(&sm),
    );
    output_events.clear();

    // ── Step 8: Button — concurrent transitions in two orthogonal regions ──
    // Both Audio (Playing→Paused) and Screen (Bright→Dim) have a Button edge,
    // so BOTH fire this step. Active itself has no Button edge and merely
    // persists, so its `during` (ActiveTick) still fires first. Region order is
    // the tuple order: Audio before Screen. (This also parks Audio in Paused,
    // which has no Fault handler — setup for the next step.)
    let before = config(&sm);
    sm.step(InputEvent::Button, &healthy, &mut output_events);
    show(
        "step 8 — Button: Audio and Screen transition concurrently",
        &before,
        &output_events.order,
        &config(&sm),
    );
    output_events.clear();

    // ── Step 9: Fault while Audio is Paused — propagates to Active→Off ─────
    // Paused has no Fault edge, and Screen has none either, so no descendant
    // handles Fault. It falls through to the Active→Off edge: hard crash. Off
    // is a trap state with no during and no outgoing edges.
    let before = config(&sm);
    sm.step(InputEvent::Fault, &healthy, &mut output_events);
    show(
        "step 9 — Fault while Paused: propagates up to Active->Off (crash)",
        &before,
        &output_events.order,
        &config(&sm),
    );
    output_events.clear();

    // ── Step 10: a fresh machine, driven by `execute` with a multi-event queue ─
    // `StateMachineRoot::execute` steps once per truthy event, in *enum order*
    // (= input-event priority). We queue {Sleep, Tick} together: Sleep (index
    // 1) is processed before Tick (index 4). So in ONE call the machine first
    // goes Active→Standby on Sleep, then takes a Standby Tick — two run-to-
    // completion steps in a single timestep.
    let mut trace = Events::default();
    let mut sm2 = StateMachine::create(build(), &mut trace);
    trace.clear();
    let queue = enum_map! {
        InputEvent::Fault => false,
        InputEvent::Sleep => true,   // processed first (lower enum index)
        InputEvent::Wake  => false,
        InputEvent::Button => false,
        InputEvent::Tick  => true,   // processed second
    };
    let before = config(&sm2);
    sm2.execute(queue, &healthy, &mut trace);
    show(
        "step 10 — execute({Sleep,Tick}) drains the queue in priority order",
        &before,
        &trace.order,
        &config(&sm2),
    );
    trace.clear();
    println!(
        "    (Sleep fired Active->Standby, THEN Tick ran inside Standby — \
         two steps, one execute call)"
    );

    // ── Bonus: the counting sink from the library ─────────────────────────
    // `Events<Out>` (the provided default sink) keeps a per-event count, which
    // is exactly what a block-diagram output port reports.
    let mut counts = Events::<OutputEvent>::default();
    let mut sm3 = StateMachine::create(build(), &mut counts);
    for _ in 0..3 {
        sm3.step(InputEvent::Tick, &healthy, &mut counts);
    }
    println!("\n▶ bonus — Events counting sink after boot + 3 healthy Ticks");
    println!(
        "    Heartbeat count: {}",
        counts.counts[OutputEvent::Heartbeat]
    );
    println!(
        "    Decoding  count: {}",
        counts.counts[OutputEvent::Decoding]
    );
    println!(
        "    Rendering count: {}",
        counts.counts[OutputEvent::Rendering]
    );
}
