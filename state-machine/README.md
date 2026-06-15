# State Machine

A `no_std`, allocation-free hierarchical state machine (statechart) library for the Pictorus platform.

It implements the familiar statechart model — nested states, orthogonal (parallel) regions, guarded transitions, and entry/exit/during actions — but is structured to fit Pictorus' atomic time-step model of computation: a machine is advanced exactly once per tick, reads an immutable bundle of input data, and reports what it did by emitting output events.

## Model

A machine is a tree of *state diagrams*. Each diagram has its own enum of states and tracks one active state at a time. A state can nest:

- **Child diagrams** to form a hierarchy (a parent state contains a sub-machine), and/or
- **Orthogonal regions** — multiple child diagrams that are all active at once and step concurrently.

The whole tree shares one `InputEvent` enum, one `InputData` type, and one `OutputEvent` enum.

### Behavior on each step

`StateMachine::step` takes a single input event plus the current input data and runs in two passes:

1. **Select.** Walk the active tree and pick at most one transition per diagram. Selection is **child-first**: a deeper handler preempts its ancestors, so a region can absorb an event before it reaches the top. A transition is chosen by matching the event, then taking the first edge whose guard passes (edge order is priority).
2. **Execute.** Apply the selected transitions, emitting output events in statechart order: exit actions bottom-up, then the transition action, then entry actions top-down (cascading into the target's default states). Diagrams that did not transition emit their `during` action instead.

A transition with no target is an *internal transition*: its action fires and `during` still runs, but no state change or entry/exit occurs.

Output events are pushed to an `EventSink`. The provided `Events` sink keeps a per-event count (and, with the `event-log` feature, an ordered log), which maps directly onto a block's output ports.

## Defining a machine

Each atomic diagram is described by implementing `StateDiagramSpec`:

- associated `State` / `InputEvent` / `InputData` / `OutputEvent` types,
- a `const TRANSITIONS` table (state -> event -> ordered, optionally-guarded edges),
- optional `on_enter` / `on_exit` / `during` hooks returning an output event, and
- a `default_transition` naming the initial state.

Diagrams are composed into a tree using tuples for parallel regions and the `children!` macro to declare which states nest sub-machines. States with no children use the `new_all_simple_states` constructor. The assembled tree is wrapped in a `StateMachine`, which is the runtime entry point (`create`, `step`, and `execute` for draining a queue of events in priority order).

## Features

- `event-log` *(default)* — record the ordered list of emitted events in `Events` (implies `alloc`).
- `alloc` — enable allocation-backed conveniences.
- `std` — pull in `std` (implies `alloc`); off by default.

The core library is `no_std` and needs no heap.

## Examples

Two heavily commented examples walk through the full semantics:

```
cargo run --example audio_player   --features event-log
cargo run --example nested_drone   --features event-log
```

`audio_player` is the recommended starting point — each step in `main` demonstrates one feature (priority resolution, guards, internal transitions, child-first preemption, concurrent region transitions, and more) and prints the emitted event stream against a commentary of what the engine did.