# Pictorus FFI

The runtime and peripheral adapters behind the Pictorus C interface.

Exporting a Pictorus app as a C library produces a static library, `ffi_lib`, that wraps the
app's generated, target independent Rust library in `extern "C"` functions. This crate is what
`ffi_lib` builds on. It is not meant to be depended on by hand.

The C caller owns the hardware. It initializes its peripherals and hands the library small sets
of functions that read a pin, do an I2C transaction, and so on. Everything the model expects of
a peripheral beyond that, such as reading an ADC once per tick or tracking staleness, stays in
Rust, so an app behaves the same here as on a native target.

## Contents

```
include/pictorus_ffi.h   the fixed header: status codes, peripheral handles, the runtime
src/runtime.rs           pictorus_rt_init, the heap, the panic handler, the critical section
src/peripherals/         one adapter per peripheral kind, implementing the block I/O traits
src/size.rs              the macro that exports the size and alignment of an app instance
src/logger.rs            a logger that never logs
examples/mock_ffi_lib.rs a hand-written stand-in for a generated ffi_lib
tests/                   a C program that is built and run against that example
```

`pictorus_ffi.h` is the same for every app and is written by hand. The per-app header,
`app_interface.h`, is generated and declares the peripherals that app uses, its inputs and
outputs, and the functions that create, step and tear down an instance.

## Using an exported library

```c
#include "pictorus_ffi.h"
#include "app_interface.h"

pictorus_rt_init(&(pictorus_rt_config_t){ .heap = heap, .heap_bytes = sizeof heap });

AppPeripherals peripherals = {
    .pwr_btn = { .ctx = &BTN0, .read = btn_read },
    .imu_bus = { .ctx = &I2C1, .write_read = imu_write_read, .write = imu_write },
};

AppInterface *app;
app_interface_init(app_mem, &peripherals, &app);

for (;;) {
    wait_for_tick(APP_INTERFACE_FUNDAMENTAL_TIMESTEP_US);
    app_interface_step(app, now_us(), &inputs, &outputs);
}
```

`tests/c/test_runtime.c` is a complete, working version of this against the mock library.

## The runtime

`pictorus_rt_init` must be called once, before any app instance is created. It takes:

- **A heap.** Every app instance in a binary shares one allocator, because Rust has no
  per-instance allocators, so only the integrator knows how much memory is needed and where
  it should live. The heap may be `NULL` if the app does not allocate. Where there is an
  operating system, the system allocator is used and the heap is ignored.
- **`on_panic`.** Called with a description of an internal panic, truncated to 128 bytes.
- **A critical section.** See below.

### Panics

A Rust panic handler cannot return. When the library panics it calls `on_panic`, if one was
given, and then spins forever. That makes `on_panic` the place for the integrator to put the
system in a safe state: stop actuators, record the message, and reset or halt as appropriate.
If it returns, or there isn't one, the library spins. Where there is an operating system, the
standard library's panic handling applies instead and `on_panic` is ignored.

Build the library with `panic = "abort"`.

### The critical section

If app instances are used from more than one task or interrupt, give `pictorus_rt_init` a
`critical_section_enter` and `critical_section_exit` pair. The library calls them around each
allocation and, on targets with no atomic read-modify-write instructions such as Cortex-M0,
around its own atomic operations. What they do is the integrator's choice. Masking interrupts is
typical, and then interrupts stay masked for as long as the protected code runs, which for an
allocation is not constant time. `enter` returns an opaque state that is handed back to `exit`,
so sections can nest.

Without them the library is unprotected, and all calls into it must come from one context.

## Peripherals

Every peripheral kind has a fixed handle type in `pictorus_ffi.h`: a context pointer plus the
functions for that kind. The integrator writes the handles once per board and reuses them across
apps.

| Kind | Operations | Notes |
|---|---|---|
| GPIO in | `read` | Not cached. Reads as `1.0` or `0.0` in the model. |
| GPIO out | `write` | |
| PWM | `set` | Two channels. Called only when the values change. |
| ADC | `read` | At most once per tick. |
| DAC | `write` | Values are passed through, in channel order. |
| I2C | `write_read`, `write` | Not cached, since devices share a bus. |
| SPI | `read`, `write` | Chip select is the caller's, per call. `read` is once per tick. |
| Serial | `read`, `write` | `read` must not block, and is once per tick. |
| CAN | `receive`, `transmit` | The receive queue is drained once per tick. |
| UDP | `recv`, `send` | `recv` is drained once per tick and the latest datagram is kept. |

Settings a model chooses for a peripheral, such as a baud rate, are declared as
`pictorus_*_config_t` structs. The generated header has a `static const` instance for each
peripheral that has any.

### Failures

A failed operation never changes what the model does. It is treated as "no new data": an input
keeps its previous value, or yields an empty byte slice, and an output is retried on the next
tick.

Failures are counted per named peripheral. `app_interface_step` returns `PICTORUS_ERR_PERIPHERAL`
if any operation failed during the tick, and `app_interface_peripheral_status` reports the last
error and a count for each peripheral. A function left `NULL` counts as a failing operation.

## Memory

An app instance is created in memory the caller provides, so several instances of one app need
nothing more than a region each. The size and alignment are available at run time from
`app_interface_size` and `app_interface_align`. They are also exported as the absolute symbols
`PICTORUS_APP_INTERFACE_SIZE` and `PICTORUS_APP_INTERFACE_ALIGN`, so that a linker script can
reserve space for instances. The values are read from the built library, for example with
`nm -S`.

Buffers the library hands to C belong to the library. C uses them during the call and never
keeps the pointer.

### Without an allocator

The `alloc` feature is required. Byte buffers all live in one type, `ByteBuffer`, so a build
without an allocator is a change to that type: a fixed-capacity buffer sized from the target's
`buffer_size_bytes`. The blocks that buffer data themselves (I2C, serial, SPI and CAN) still
allocate, so that is not the only change it needs.

## Targets and features

- `alloc` is the only feature. It enables the blocks that need a heap. Where there is an
  operating system, nothing else is needed.
- On bare metal (`target_os = "none"`) the crate also provides the global allocator (with
  `alloc`), the panic handler and the critical section. Where there is an operating system it
  provides none of them.
- Cortex-M0 builds work, with atomic operations going through the critical section.

## Testing

```sh
cargo test -p pictorus-ffi --features alloc
```

runs the unit tests and then the C test. `tests/c_integration.rs` builds `tests/c/test_runtime.c`
with the system C compiler, links it against the `mock_ffi_lib` example (a staticlib that Cargo
builds along with the tests), and runs it. The test needs a C compiler; set `CC` to choose one.
The example is arranged the way generated code is, so it also shows how the pieces here are
composed into a library.

## Not yet covered

Struct and enum types in an app's top-level inputs and outputs, logging, reading and updating
parameters from C, and several apps in one binary.
