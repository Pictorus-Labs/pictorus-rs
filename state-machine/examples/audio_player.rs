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

use state_machine::*;

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
impl StateMachineSpec for PlayerSpec {
    type State = Player;
    type InputEvent = InputEvent;
    type InputData = InputData;
    type OutputEvent = OutputEvent;

    const EDGES: EdgeTable<Player, InputEvent, InputData, OutputEvent> = &[
        (
            Player::Active,
            &[
                (
                    InputEvent::Sleep,
                    &[(
                        Some(|c: &InputData| c.battery > 0),
                        Some(OutputEvent::GoStandby),
                        Some(Player::Standby),
                    )],
                ),
                (
                    InputEvent::Fault,
                    &[(None, Some(OutputEvent::Crashed), Some(Player::Off))],
                ),
                (
                    InputEvent::Tick,
                    &[
                        // priority 1 — critical: empty battery overrides everything
                        (
                            Some(|c: &InputData| c.battery == 0),
                            Some(OutputEvent::BatteryDead),
                            Some(Player::Off),
                        ),
                        // priority 2 — low and unplugged: drop to Standby
                        (
                            Some(|c: &InputData| c.battery < 20 && !c.charging),
                            Some(OutputEvent::AutoSleep),
                            Some(Player::Standby),
                        ),
                        // priority 3 — catch-all internal tick (no target => internal)
                        (None, Some(OutputEvent::Heartbeat), None),
                    ],
                ),
            ],
        ),
        (
            Player::Standby,
            &[
                (
                    InputEvent::Wake,
                    &[(None, Some(OutputEvent::Waking), Some(Player::Active))],
                ),
                (
                    InputEvent::Fault,
                    &[(None, Some(OutputEvent::Crashed), Some(Player::Off))],
                ),
                (
                    InputEvent::Tick,
                    &[
                        (
                            Some(|c: &InputData| c.battery == 0),
                            Some(OutputEvent::BatteryDead),
                            Some(Player::Off),
                        ),
                        // internal trickle-charge tick, only while charging
                        (
                            Some(|c: &InputData| c.charging),
                            Some(OutputEvent::Trickle),
                            None,
                        ),
                        // (if neither guard holds, Tick matches no edge => discarded)
                    ],
                ),
            ],
        ),
        // Player::Off has no outgoing edges: it is a trap state.
    ];

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
impl StateMachineSpec for AudioSpec {
    type State = Audio;
    type InputEvent = InputEvent;
    type InputData = InputData;
    type OutputEvent = OutputEvent;

    const EDGES: EdgeTable<Audio, InputEvent, InputData, OutputEvent> = &[
        (
            Audio::Playing,
            &[
                (
                    InputEvent::Button,
                    &[(None, Some(OutputEvent::Pause), Some(Audio::Paused))],
                ),
                // Internal: while Playing, the Audio region ABSORBS a Fault
                // (e.g. transient decoder hiccup) instead of letting it bubble
                // up to the Active→Off edge. Because a deeper handler wins, the
                // top-level Fault transition is never even evaluated.
                (
                    InputEvent::Fault,
                    &[(None, Some(OutputEvent::AudioRecover), None)],
                ),
            ],
        ),
        (
            // Paused has NO Fault edge — so a Fault while Paused is NOT absorbed
            // here and propagates up to the Active→Off transition.
            Audio::Paused,
            &[(
                InputEvent::Button,
                &[(None, Some(OutputEvent::Resume), Some(Audio::Playing))],
            )],
        ),
    ];

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
impl StateMachineSpec for ScreenSpec {
    type State = Screen;
    type InputEvent = InputEvent;
    type InputData = InputData;
    type OutputEvent = OutputEvent;

    const EDGES: EdgeTable<Screen, InputEvent, InputData, OutputEvent> = &[
        (
            Screen::Bright,
            &[(
                InputEvent::Button,
                &[(None, Some(OutputEvent::DimScreen), Some(Screen::Dim))],
            )],
        ),
        (
            Screen::Dim,
            &[(
                InputEvent::Button,
                &[(None, Some(OutputEvent::BrightScreen), Some(Screen::Bright))],
            )],
        ),
    ];

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
impl StateMachineSpec for PowerSpec {
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
impl StateMachineSpec for NetSpec {
    type State = Net;
    type InputEvent = InputEvent;
    type InputData = InputData;
    type OutputEvent = OutputEvent;

    const EDGES: EdgeTable<Net, InputEvent, InputData, OutputEvent> = &[
        (
            Net::Listening,
            &[(
                InputEvent::Button,
                &[(None, Some(OutputEvent::NetConnect), Some(Net::Connected))],
            )],
        ),
        (
            Net::Connected,
            &[(
                InputEvent::Button,
                &[(None, Some(OutputEvent::NetDisconnect), Some(Net::Listening))],
            )],
        ),
    ];

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
// `Active` and `Standby` have *different* region tuple types, and `Off` has no
// children at all. They must share one `C` type in `EnumMap<Player, C>`, so we
// hand-roll a unifying enum that just forwards every NodeInterface call. This
// is the small bit of boilerplate the homogeneous-EnumMap model charges for
// heterogeneous subtrees.
type Leaf<S> = LeafNode<S, InputEvent, InputData, OutputEvent>;

enum PlayerChildren {
    Active((Leaf<AudioSpec>, Leaf<ScreenSpec>)), // Audio ∥ Screen
    Standby((Leaf<PowerSpec>, Leaf<NetSpec>)),   // Power ∥ Net
    Off(NoChildren<InputEvent, InputData, OutputEvent>), // simple state: no children
}

impl NodeInterface for PlayerChildren {
    type InputEvent = InputEvent;
    type InputData = InputData;
    type OutputEvent = OutputEvent;
    fn select(&mut self, e: InputEvent, d: &InputData) -> bool {
        match self {
            PlayerChildren::Active(t) => t.select(e, d),
            PlayerChildren::Standby(t) => t.select(e, d),
            PlayerChildren::Off(t) => t.select(e, d),
        }
    }
    fn execute_pending<K: EventSink<OutputEvent>>(&mut self, sink: &mut K) {
        match self {
            PlayerChildren::Active(t) => t.execute_pending(sink),
            PlayerChildren::Standby(t) => t.execute_pending(sink),
            PlayerChildren::Off(t) => t.execute_pending(sink),
        }
    }
    fn enter<K: EventSink<OutputEvent>>(&mut self, sink: &mut K) {
        match self {
            PlayerChildren::Active(t) => t.enter(sink),
            PlayerChildren::Standby(t) => t.enter(sink),
            PlayerChildren::Off(t) => t.enter(sink),
        }
    }
    fn exit<K: EventSink<OutputEvent>>(&mut self, sink: &mut K) {
        match self {
            PlayerChildren::Active(t) => t.exit(sink),
            PlayerChildren::Standby(t) => t.exit(sink),
            PlayerChildren::Off(t) => t.exit(sink),
        }
    }
    fn reset(&mut self) {
        match self {
            PlayerChildren::Active(t) => t.reset(),
            PlayerChildren::Standby(t) => t.reset(),
            PlayerChildren::Off(t) => t.reset(),
        }
    }
}

type PlayerNode = Node<PlayerSpec, PlayerChildren>;

fn build() -> PlayerNode {
    Node::new(enum_map! {
        Player::Active  => PlayerChildren::Active((Node::new_leaf(), Node::new_leaf())),
        Player::Standby => PlayerChildren::Standby((Node::new_leaf(), Node::new_leaf())),
        Player::Off     => PlayerChildren::Off(NoChildren::default()),
    })
}

// ─── A sink that records emission order so we can print it ─────────────────
// TODO: Just use the built in feature flag for this!
#[derive(Default)]
struct Trace(Vec<OutputEvent>);
impl EventSink<OutputEvent> for Trace {
    fn emit(&mut self, e: OutputEvent) {
        self.0.push(e);
    }
}
impl Trace {
    fn drain(&mut self) -> Vec<OutputEvent> {
        std::mem::take(&mut self.0)
    }
}

// ─── Inspecting the live configuration (purely for the printout) ───────────
fn config(sm: &StateMachineRoot<PlayerNode>) -> String {
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

fn show(label: &str, before: &str, emitted: Vec<OutputEvent>, after: &str) {
    println!("\n▶ {label}");
    println!("    config: {before}  ->  {after}");
    println!("    emitted: {emitted:?}");
}

fn main() {
    // ── Boot ────────────────────────────────────────────────────────────
    // `create` runs the top default transition, then cascades default
    // transitions down through both of Active's regions to stable leaves.
    let mut trace = Trace::default();
    let mut sm = StateMachineRoot::create(build(), &mut trace);
    println!("▶ boot (create)");
    println!("    config: <none>  ->  {}", config(&sm));
    println!("    emitted: {:?}", trace.drain());
    println!(
        "    (top default action, top on_enter, then each region's default \
         action + on_enter, top-down)"
    );

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
    sm.step(InputEvent::Tick, &healthy, &mut trace);
    show(
        "step 1 — Tick @80%: priority falls through to the internal heartbeat",
        &before,
        trace.drain(),
        &config(&sm),
    );

    // ── Step 2: Tick, critically low + unplugged → external transition ─────
    // Now edge 2's guard (batt<20 && !charging) passes and edge 2 fires: an
    // EXTERNAL transition Active→Standby. We're still in Playing/Bright, whose
    // states carry exit actions, so the shape is plain to see: exit is
    // bottom-up (AudioStop, ScreenBrightExit, then ActiveExit), then the
    // transition action, then entry top-down (Standby, then each region's
    // default cascade). No `during` anywhere — this step *is* an external
    // transition at the top.
    let before = config(&sm);
    sm.step(InputEvent::Tick, &low, &mut trace);
    show(
        "step 2 — Tick @10% unplugged: external Active->Standby (exit↑, action, enter↓)",
        &before,
        trace.drain(),
        &config(&sm),
    );

    // ── Step 3: Tick in Standby, not charging → discarded, but `during` runs ─
    // In Standby, Tick edge 1 (batt==0) fails and edge 2 (charging) fails, so
    // Tick matches NO edge and is "silently discarded" as a transition. The
    // engine still walks the active tree and fires `during` at every level.
    let before = config(&sm);
    sm.step(InputEvent::Tick, &low, &mut trace);
    show(
        "step 3 — Tick in Standby, unplugged: no edge matches, yet `during` fires",
        &before,
        trace.drain(),
        &config(&sm),
    );

    // ── Step 4: Tick in Standby while charging → internal trickle ──────────
    // Same state, different data: edge 2's guard (charging) now passes, firing
    // an internal transition. `during` still fires at each level; the internal
    // action (Trickle) is ordered right after Standby's own `during`.
    let charging = InputData {
        battery: 10,
        charging: true,
    };
    let before = config(&sm);
    sm.step(InputEvent::Tick, &charging, &mut trace);
    show(
        "step 4 — Tick in Standby, charging: internal trickle (during + internal action)",
        &before,
        trace.drain(),
        &config(&sm),
    );

    // ── Step 5: Button in Standby — region transitions, ancestor persists ──
    // Only the Net region has a Button edge (Listening→Connected). Standby and
    // the Power region persist, so their `during`s fire; Net does a real
    // external transition (on_exit Listening, action, on_enter Connected) and
    // therefore does NOT fire its own `during`.
    let before = config(&sm);
    sm.step(InputEvent::Button, &low, &mut trace);
    show(
        "step 5 — Button in Standby: Net transitions while Standby/Power persist",
        &before,
        trace.drain(),
        &config(&sm),
    );

    // ── Step 6: Wake → back to Active (fresh region defaults) ──────────────
    let before = config(&sm);
    sm.step(InputEvent::Wake, &healthy, &mut trace);
    show(
        "step 6 — Wake: Standby->Active, regions re-enter via their defaults",
        &before,
        trace.drain(),
        &config(&sm),
    );

    // ── Step 7: Fault while Audio is Playing — absorbed by the child ───────
    // Audio::Playing has a Fault edge (internal AudioRecover). Child-first
    // selection means a deeper handler wins, so the Active→Off Fault edge is
    // never evaluated: the fault is absorbed and we stay in Active.
    let before = config(&sm);
    sm.step(InputEvent::Fault, &healthy, &mut trace);
    show(
        "step 7 — Fault while Playing: ABSORBED by Audio (deeper handler wins)",
        &before,
        trace.drain(),
        &config(&sm),
    );

    // ── Step 8: Button — concurrent transitions in two orthogonal regions ──
    // Both Audio (Playing→Paused) and Screen (Bright→Dim) have a Button edge,
    // so BOTH fire this step. Active itself has no Button edge and merely
    // persists, so its `during` (ActiveTick) still fires first. Region order is
    // the tuple order: Audio before Screen. (This also parks Audio in Paused,
    // which has no Fault handler — setup for the next step.)
    let before = config(&sm);
    sm.step(InputEvent::Button, &healthy, &mut trace);
    show(
        "step 8 — Button: Audio and Screen transition concurrently",
        &before,
        trace.drain(),
        &config(&sm),
    );

    // ── Step 9: Fault while Audio is Paused — propagates to Active→Off ─────
    // Paused has no Fault edge, and Screen has none either, so no descendant
    // handles Fault. It falls through to the Active→Off edge: hard crash. Off
    // is a trap state with no during and no outgoing edges.
    let before = config(&sm);
    sm.step(InputEvent::Fault, &healthy, &mut trace);
    show(
        "step 9 — Fault while Paused: propagates up to Active->Off (crash)",
        &before,
        trace.drain(),
        &config(&sm),
    );

    // ── Step 10: a fresh machine, driven by `execute` with a multi-event queue ─
    // `StateMachineRoot::execute` steps once per truthy event, in *enum order*
    // (= input-event priority). We queue {Sleep, Tick} together: Sleep (index
    // 1) is processed before Tick (index 4). So in ONE call the machine first
    // goes Active→Standby on Sleep, then takes a Standby Tick — two run-to-
    // completion steps in a single timestep.
    let mut trace = Trace::default();
    let mut sm2 = StateMachineRoot::create(build(), &mut trace);
    trace.drain();
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
        trace.drain(),
        &config(&sm2),
    );
    println!(
        "    (Sleep fired Active->Standby, THEN Tick ran inside Standby — \
         two steps, one execute call)"
    );

    // ── Bonus: the counting sink from the library ─────────────────────────
    // `Events<Out>` (the provided default sink) keeps a per-event count, which
    // is exactly what a block-diagram output port reports.
    let mut counts = Events::<OutputEvent>::default();
    let mut sm3 = StateMachineRoot::create(build(), &mut counts);
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
