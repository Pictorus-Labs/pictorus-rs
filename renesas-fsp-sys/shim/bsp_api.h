/*
 * Stand-in for the FSP BSP, used only when generating Rust bindings.
 *
 * Every FSP interface header opens with `#include "bsp_api.h"`, and the real
 * one reaches into per-MCU files that do not exist until a user configures an
 * e2 studio project. This supplies the small part of the BSP that actually
 * reaches an interface type, so that bindgen can compute struct layouts.
 *
 * Nothing here is compiled into a real build -- the user's project builds the
 * genuine FSP sources against the genuine BSP. The layouts must match, which is
 * why each type below is an enum carrying the real one's extreme enumerators
 * rather than a fixed-width typedef: under `-fshort-enums` an enum re-derives
 * its width from the same rule the C side used, so the two cannot drift.
 *
 * See README.md -- "The BSP shim" and "ABI" -- for the reasoning and for what
 * a width mismatch costs.
 */

#ifndef BSP_API_H
#define BSP_API_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "fsp_common_api.h"

FSP_HEADER

/* Cortex-M interrupt number, from the BSP's bsp_exceptions.h. The full range is
 * reproduced because the range is what sets the width. Renesas peripheral IRQs
 * do not widen it: the generated vector_data.h emits them as casts
 * (`#define VECTOR_NUMBER_GPT0_COUNTER_OVERFLOW ((IRQn_Type) 12)`), not as new
 * enumerators, so the range stays -15..-1 and the type stays one signed byte. */
typedef enum IRQn
{
    Reset_IRQn            = -15,
    NonMaskableInt_IRQn   = -14,
    HardFault_IRQn        = -13,
    MemoryManagement_IRQn = -12,
    BusFault_IRQn         = -11,
    UsageFault_IRQn       = -10,
    SecureFault_IRQn      = -9,
    SVCall_IRQn           = -5,
    DebugMonitor_IRQn     = -4,
    PendSV_IRQn           = -2,
    SysTick_IRQn          = -1,
} IRQn_Type;

/* Levels for an individual pin, from the BSP's bsp_io.h. */
typedef enum e_bsp_io_level
{
    BSP_IO_LEVEL_LOW  = 0,
    BSP_IO_LEVEL_HIGH = 1,
} bsp_io_level_t;

/* Port identity: (port << 8), from bsp_io.h's superset list. Only the endpoints
 * are reproduced; the ports between are consecutive and change nothing. */
typedef enum e_bsp_io_port
{
    BSP_IO_PORT_00 = 0x0000,
    BSP_IO_PORT_14 = 0x0E00,
} bsp_io_port_t;

/* Pin identity: (port << 8) | pin, from bsp_io.h's superset list. Do not drop
 * the unused `BSP_IO_PORT_FF_PIN_FF` sentinel -- it is what fixes this type at
 * two bytes on every RA part. */
typedef enum e_bsp_io_port_pin_t
{
    BSP_IO_PORT_00_PIN_00 = 0x0000,
    BSP_IO_PORT_FF_PIN_FF = 0xFFFF,
} bsp_io_port_pin_t;

/* Four bytes with or without `-fshort-enums`, since 1000000 does not fit in
 * fewer -- hence no static assertion for this one below. */
typedef enum e_bsp_delay_units
{
    BSP_DELAY_UNITS_SECONDS      = 1000000,
    BSP_DELAY_UNITS_MILLISECONDS = 1000,
    BSP_DELAY_UNITS_MICROSECONDS = 1,
} bsp_delay_units_t;

/* Do not add R_BSP_PinRead, R_BSP_PinWrite, R_BSP_PinCfg or
 * R_BSP_PinAccessEnable/Disable here: they are __STATIC_INLINE with no
 * out-of-line fallback, so they emit no symbol and bindgen produces nothing.
 * Reach a pin through the ioport vtable's pinRead/pinWrite instead. The two
 * below are declared because they are real linkable symbols. */
void R_BSP_SoftwareDelay(uint32_t delay, bsp_delay_units_t units);

fsp_err_t R_FSP_VersionGet(fsp_pack_version_t * const p_version);

/* Sizes as FSP's own toolchains see them. These hold under `-fshort-enums`;
 * drop the flag from build.rs and they fire, which is the intent. build.rs
 * blocklists the names so they stay out of the crate's public API. */
#define FSP_SYS_STATIC_ASSERT(cond, name)    typedef char name[(cond) ? 1 : -1]
FSP_SYS_STATIC_ASSERT(sizeof(IRQn_Type) == 1, fsp_sys_irqn_size);
FSP_SYS_STATIC_ASSERT(sizeof(bsp_io_level_t) == 1, fsp_sys_io_level_size);
FSP_SYS_STATIC_ASSERT(sizeof(bsp_io_port_t) == 2, fsp_sys_port_size);
FSP_SYS_STATIC_ASSERT(sizeof(bsp_io_port_pin_t) == 2, fsp_sys_port_pin_size);

FSP_FOOTER

#endif
