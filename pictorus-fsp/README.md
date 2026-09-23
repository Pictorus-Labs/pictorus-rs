# Pictorus FSP

Pictorus platform drivers for Renesas RA parts running under the Flexible
Software Package that is built on [`renesas-fsp-sys`](../renesas-fsp-sys/).

## Shape

```
c/rm_pictorus_app.h      the C/Rust seam, canonical copy
c/rm_pictorus_app.c      the FSP-side shim: heap, time base, tick loop
c/shim/                  a stand-in for the configurator-generated cfg header,
                         so bindgen has something to parse
build.rs                 bindgen over the seam header only
bindings/seam.rs         committed snapshot; tests/snapshot.rs guards it
src/error.rs             FspError, a newtype over fsp_err_t
src/diag.rs              warn_once! — one report per call site, ever
src/app.rs               seam bindings and pictorus_rt_bind
src/peripherals.rs       Peripherals: the table, by model I/O index
src/gpio_protocol.rs     FspPin, Ioport, FspInputPin, FspOutputPin
src/pwm_protocol.rs      FspPwm
examples/fsp_module.rs   staticlib — the only linkable build shape
```

## Direction of control

FSP calls into Rust; Rust never overrides FSP.

```
 reset ──▸ FSP startup ──▸ R_BSP_WarmStart(POST_C)     IOPORT opened here
                              │                        
                              ▼                         
                           main() ──▸ hal_entry()
                              │
                              ▼
              rm_pictorus_app.c :: open()
                 ├─ opens the time-base timer, reads its clock
                 ├─ pictorus_rt_heap_init(...)   ─────▸ Rust: application crate
                 ├─ pictorus_rt_bind(&bindings)  ─────▸ Rust: src/app.rs
                 └─ app_interface_new()          ─────▸ Rust: builds IoManager
                              │                          via Peripherals::take()
                              ▼
              rm_pictorus_app.c :: run()
                 └─ per tick: app_interface_update(app, measured_seconds)
                                                 ─────▸ Rust: one model step
```

The model runs from that loop, never from the timer ISR: it allocates, may write
telemetry, and its duration is a function of the user's block diagram, so running
it at interrupt priority would make every FSP driver's latency depend on the
model.

## Indices, not names

`Peripherals` hands out wrappers by the block's position in model I/O order:

```rust,ignore
let p = Peripherals::take()?;
let led = p.gpio_output(0)?;   // first GPIO block in the model
let motor = p.pwm(0)?;         // first PWM block
```

Nothing can check that agreement, one side is a generated Rust crate, the other
the generated `hal_data.c` table, and they meet only at run time — a disagreement
drives the wrong pin and reports nothing. Generating both from the same model
description is what makes it safe.

## ABI

`c/rm_pictorus_app.h` is compiled twice, by two toolchains that never see each
other: once here into a prebuilt static library, and once in whatever e2 studio
project imported the pack. A field added, reordered or resized between those two
builds is not a compile error on either side — it is a struct read at the wrong
offsets on a board.

`bindings/seam.rs` is a committed snapshot of the bindgen output and
`tests/snapshot.rs` fails if a fresh run disagrees. 

```sh
PICTORUS_FSP_UPDATE_SNAPSHOT=1 cargo build -p pictorus-fsp
```

Only a host build may update it, for the same reason as in `renesas-fsp-sys`:
bindgen emits layout assertions for Arm targets but not for the host, and the
assertion-free form is canonical.

`rm_pictorus_bindings_t` currently carries GPIO and PWM only. Peripherals added
later arrive as `void const *` — `adc_instance_t` and friends come from headers
a project only contains when its stack selects the matching component, and this
header is reached from `hal_data.h`, so naming one would break every project
whose model does not use that peripheral. Append rather than insert, and refresh
the snapshot in the same commit.

## Building, and what a build proves

```sh
cargo test -p pictorus-fsp --all-features        # host: conversions, seam snapshot
cargo clippy -p pictorus-fsp --all-features --target thumbv8m.main-none-eabihf
cargo build -p pictorus-fsp --example fsp_module --release --all-features \
    --target thumbv8m.main-none-eabihf
```

**`thumbv8m.main-none-eabihf` — hard float.** Not the soft-float
`thumbv8m.main-none-eabi` that `pictorus-renesas` uses. Cortex-M33 RA parts have
a single-precision FPU and FSP is built `-mfloat-abi=hard -mfpu=fpv5-sp-d16`; a
soft-float Rust library links against it without complaint and corrupts every
float passed across the seam.

The `fsp_module` example is the only target that proves anything about the final
object. This crate is an rlib, and under LTO an rlib holds bitcode rather than
ELF, so its float ABI, its enum-size attribute and which symbols survive are
unobservable until something links a `staticlib`. The example is also the
template a generated application follows; everything in it a real application
also needs is marked `TEMPLATE`.

Six symbols have to be exported, and the link fails loudly if one is missing:

```sh
nm -g libfsp_module.a | grep -E ' T (app_interface|pictorus_rt)'
```

`pictorus_rt_bind` comes from this crate. `app_interface_{new,update,free}` come
from Pictorus codegen for `LibType.STATIC`. `pictorus_rt_heap_init` and
`pictorus_rt_build_id` are the hand-added delta, and the example shows both.

## Known limitations

**Pin mode is not configured, by design.** `R_IOPORT_PinCfg` is never called;
direction and pull come from the Pins tab via `pin_data.c`. A pin the user
forgot to set to Output yields a wrapper that reports success and drives
nothing, with no diagnostic available. Reminding the user belongs in the
module's Developer Assistance text.

**A PWM block's duties 2 and 3 are unused.** A Pictorus PWM block carries four
duty cycles; a GPT or AGT drives two outputs. Duty 0 maps to GTIOCA and duty 1 to
GTIOCB. A model needing four independent duties needs two timers and two PWM
blocks.

**The time base must have an interrupt priority set** in the configurator. A
blank priority allocates no vector, so the callback never fires,
`elapsed_periods` stays zero, and model time runs to the end of one timer period
and stops. `open()` checks for this and fails rather than letting it through.

**Nothing here has run on hardware, and `rm_pictorus_app.c` has never been
compiled.** No `arm-none-eabi-gcc` is available in CI or in the development
environment used so far, and the shim cannot be compiled outside a configured
project — it needs the real `bsp_api.h` and the generated
`rm_pictorus_app_cfg.h`. It is the least-verified file here. Treat its first
compile as a debugging session, not a formality.
