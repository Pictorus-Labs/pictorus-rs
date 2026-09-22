# Pictorus Renesas FSP Sys

Raw FFI declarations for the Renesas Flexible Software Package (FSP), generated
with `bindgen` from a curated subset of the FSP 6.6.0 interface headers in
`vendor/`. This crate contains declarations only; the driver implementations
built on top of it live in `pictorus-fsp`.

Coverage is IOPORT (GPIO) and timer (PWM). Other peripherals will be added later.

## How FSP drivers are shaped

FSP describes every peripheral twice.

- The **interface** says what a kind of peripheral can do, in the abstract.
  `r_ioport_api.h` describes I/O ports: read a pin, write a pin, set a
  direction. It names no registers and is identical across the RA family.
- An **instance** is one concrete driver implementing that interface.
  `r_ioport.h` is the RA IOPORT peripheral's implementation; `r_gpt.h` and
  `r_agt.h` are two different implementations of the *timer* interface.

Each interface defines four types, named consistently. For IOPORT:

| Type | Role |
|---|---|
| `ioport_cfg_t` | Configuration. `const`, and written for you by the e2 studio configurator. |
| `ioport_ctrl_t` | The driver's own state. Opaque — you only ever hold a pointer. |
| `ioport_api_t` | A **vtable**: a struct of function pointers, one per operation. |
| `ioport_instance_t` | `{ p_ctrl, p_cfg, p_api }` — the handle tying the three together. |

**This crate binds the interface layer only.** Everything in
`bindings/generated.rs` comes from `r_ioport_api.h` and `r_timer_api.h`.

### Example: reading a GPIO pin

The user configures an IOPORT instance in e2 studio, and the configurator emits
a `g_ioport` of type `ioport_instance_t` into the project's generated C. Rust
receives a pointer to it at runtime and calls through the vtable:

```rust,ignore
// `instance: *const ioport_instance_t`, handed over by the C side.
let mut level = bsp_io_level_t::BSP_IO_LEVEL_LOW;

let err = unsafe {
    let api = &*(*instance).p_api;
    let pin_read = api.pinRead.expect("ioport vtable has no pinRead");
    pin_read((*instance).p_ctrl, pin, &mut level)
};
```

Note `api.pinRead`, not a direct call to `R_IOPORT_PinRead`. That distinction is
the whole point: `pinRead` names only the interface, so the same compiled code
works with whichever implementation the user selected. A direct call names one
specific implementation, and it would not link here anyway — this crate never
sees `r_ioport.h`.

### Why the instance layer cannot be bound

It is not a matter of preference. An instance's control block names device
registers: `gpt_instance_ctrl_t` contains an `R_GPT0_Type *`, which pulls in the
~21,800-line device register header, which in turn pulls in `bsp_cfg.h`,
`vector_data.h` and the rest of the per-MCU Board Support Package (BSP). Those
files do not exist until someone configures an e2 studio project — long after
this crate has been built and shipped.

Interface headers reference no register structs at all, so their layouts hold
across the whole RA family. That is what lets one prebuilt library serve
arbitrary user projects.

## The BSP shim

Every FSP interface header opens with `#include "bsp_api.h"`, the entry point to
the BSP — and the real one reaches straight into the project-generated files
described above. So binding the interface layer requires a `bsp_api.h` that does
not exist.

Fortunately very little of the BSP actually reaches an interface type. For example, this is
what `pinWrite` needs:

```c
fsp_err_t (* pinWrite)(ioport_ctrl_t * const p_ctrl, bsp_io_port_pin_t pin, bsp_io_level_t level);
```

Two BSP typedefs, no registers. Across both headers this crate binds, the total
comes to five types and two functions, and `shim/bsp_api.h` declares them by
hand:

| Shim declaration | Where it shows up |
|---|---|
| `bsp_io_port_pin_t` | every pin argument in `ioport_api_t`; the value is `(port << 8) \| pin` |
| `bsp_io_level_t` | pin values — `pinWrite`'s input, `pinRead`'s out-param |
| `bsp_io_port_t` | whole-port operations such as `portRead` and `portWrite` |
| `IRQn_Type` | `timer_cfg_t::cycle_end_irq` |
| `bsp_delay_units_t` | the units argument to `R_BSP_SoftwareDelay` |
| `R_BSP_SoftwareDelay`, `R_FSP_VersionGet` | the only two functions this crate declares at all |

Those two functions are the only BSP entry points here because they are the only
ones that emit a linkable symbol. `R_BSP_PinRead`, `R_BSP_PinWrite` and their
neighbours are `__STATIC_INLINE` with no out-of-line fallback, so bindgen
produces nothing for them — reach a pin through the `ioport_api_t` vtable
instead.

`shim/assert.h` is a stub for the one libc header `fsp_common_api.h` includes,
which `-nostdlibinc` otherwise removes.

Neither shim file is ever compiled into a real build. The user's project builds
the genuine FSP sources against the genuine BSP; these exist only so bindgen can
compute struct layouts — which makes it essential that the layouts agree.

## ABI

Two toolchain settings have to match FSP's, and neither one announces itself
when it is wrong.

### `-fshort-enums`

By default a C enum is `int`-sized, four bytes, whatever its values. Under
`-fshort-enums` it shrinks to the narrowest integer its enumerators fit in, so
`bsp_io_level_t` — whose only values are 0 and 1 — becomes one byte.

FSP is built this way: every Renesas prebuilt library carries
`Tag_ABI_enum_size = small`. clang does *not* default to it for bare-metal Arm,
so `build.rs` passes the flag explicitly.

The cost of getting this wrong is not a type error, it is a silently wrong
struct. `timer_cfg_t` holds three enums ahead of its callback pointer:

| | `-fshort-enums` | default |
|---|---|---|
| offset of `p_callback` | 20 | 24 |
| `sizeof(timer_cfg_t)` | 32 | 36 |

Rust would write the callback pointer where C reads padding. `shim/bsp_api.h`
guards against this by static-asserting four of the five widths, so dropping the
flag fails the build instead. `bsp_delay_units_t` is the exception — its largest
value is 1000000, so it needs four bytes either way.

It is also why each shim type is declared as an `enum` carrying the real one's
extreme values rather than as a fixed-width `typedef`. An enum re-derives its
width from the same rule the C side used, so the two cannot drift apart. That is
what the otherwise-unused `BSP_IO_PORT_FF_PIN_FF` enumerator is doing: it is what
pins `bsp_io_port_pin_t` at two bytes.

### Hard float

A soft-float target passes `f32`/`f64` in general-purpose registers; a hard-float
one passes them in VFP registers. The two are not interchangeable, and the linker
does not object.

Cortex-M33 RA parts have a single-precision FPU, and FSP is built
`-mfloat-abi=hard -mfpu=fpv5-sp-d16`. The target is therefore
`thumbv8m.main-none-eabihf` — note that `pictorus-renesas` still uses the
soft-float `thumbv8m.main-none-eabi`, which would corrupt every float-passing
call into FSP.

## Linking

Nothing here links. There is no `links` key, no `cc` invocation and no
`cargo:rustc-link-*` — this crate emits declarations and never asks cargo to
resolve them.

So `cargo build` succeeding says nothing about whether the FSP symbols exist; it
succeeds with every reference dangling. Resolution happens in the e2 studio
project, which compiles the FSP C and links it against the static library built
from this crate.

## The bindings snapshot

`bindings/generated.rs` is a committed copy of the generated output, and
`tests/snapshot.rs` fails if a fresh run disagrees with it.

It exists because nothing in this repository compiles the FSP C. If a pack
upgrade inserted one field into `timer_cfg_t`, there would be no compile error
anywhere — just a library that links cleanly and reads every subsequent field
from the wrong offset. The snapshot diff is the only place that becomes visible.

So a failing snapshot test is not necessarily a defect; it reports that the
declarations moved. Read the diff, then accept it with:

```sh
RENESAS_FSP_SYS_UPDATE_SNAPSHOT=1 cargo build -p renesas-fsp-sys
```

Only a host build may update it. Bindgen emits layout assertions for Arm targets
but not for the host, and the assertion-free host output is the canonical form —
so a cross build refuses the update rather than committing the larger variant.

## Re-vendoring, and adding a peripheral

`vendor/` holds the FSP interface headers unmodified, in their upstream
`ra/fsp/inc/` layout, with `vendor/FSP_VERSION` recording which pack they came
from. Both procedures below start from a pack distribution zip — download it
from the [Renesas FSP releases page](https://github.com/renesas/fsp/releases),
or copy it out of an e2 studio installation under
`internal/projectgen/ra/packs/` — and both are run from this crate's root.

### Upgrading to a new FSP release

```sh
cd renesas-fsp-sys
script/vendor-headers.sh ~/Downloads/FSP_Packs_v6.7.0.zip
RENESAS_FSP_SYS_UPDATE_SNAPSHOT=1 cargo build -p renesas-fsp-sys
git diff vendor/ bindings/generated.rs
```

The script rewrites `vendor/fsp/inc/` and `vendor/FSP_VERSION`; the build
rewrites `bindings/generated.rs`. Read the snapshot diff before committing — it
is the only place a shifted struct layout shows up, since nothing here compiles
the FSP C. The build must be a host build (see above), and `src/lib.rs` will
fail its test if `FSP_VERSION` and the generated `fsp_version.h` constants
disagree.

### Adding a peripheral

Four coordinated edits — miss one and you get either a stale binding or a
header that fails to parse:

1. add its `r_*_api.h` to the `HEADERS` array in `script/vendor-headers.sh`,
   e.g. `r_spi_api.h`;
2. re-run the script against the *same* pack version already recorded in
   `vendor/FSP_VERSION`, so the new header is the only change:
   `script/vendor-headers.sh ~/Downloads/FSP_Packs_v6.6.0.zip`;
3. add `#include "r_spi_api.h"` to `wrapper.h`;
4. refresh the snapshot and review the diff:
   `RENESAS_FSP_SYS_UPDATE_SNAPSHOT=1 cargo build -p renesas-fsp-sys`.

If the new header names a BSP type `shim/bsp_api.h` does not declare, step 4
fails to parse — declare it there as an `enum` carrying the real type's extreme
values, following the existing ones.

Two peripherals need more than that, and both were in scope for the prototype
this crate came from:

- **ADC.** `adc_info_t` names an `elc_event_t`, and unlike the five shim types
  its width is genuinely per-part — two bytes on an RA4M2, but one on a device
  with fewer than 256 events. There is no single correct shim declaration.
- **CAN.** `can_frame_t`'s `data[]` is 8 bytes under `r_can.h` and 64 under
  `r_canfd.h`, so the two headers must never reach one translation unit.
