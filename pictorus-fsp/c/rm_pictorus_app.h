/*
 * rm_pictorus_app.h -- the C/Rust seam for a Pictorus application hosted by FSP.
 *
 * This is the canonical copy, living in the Rust workspace, is bindgen'd by
 * `pictorus-fsp`, and is copied verbatim into the generated CMSIS pack by
 * whatever builds that pack. 
 *
 * Shape
 * -----
 * The module presents itself as an ordinary FSP module: `rm_pictorus_cfg_t`,
 * `rm_pictorus_ctrl_t`, `rm_pictorus_api_t` and `rm_pictorus_instance_t`, the
 * same four types every FSP driver has.
 *
 * Direction
 * ---------
 * FSP calls into Rust, Rust never overrides the FSP. FSP owns the reset vector, the
 * vector table, `main()` and the clock tree. The C shim shipped alongside this
 * header owns the heap buffer and the tick loop, and calls the Rust entry
 * points declared at the bottom of this file.
 *
 * What must stay true
 * -------------------
 * Every field here is part of an ABI between separately compiled artifacts: a
 * prebuilt Rust static library and C that the user's e2 studio project compiles
 * fresh. 
 *
 */

#ifndef RM_PICTORUS_APP_H
#define RM_PICTORUS_APP_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

/*
 * Only the interfaces this module unconditionally requires.
 *
 * IOPORT and the time-base timer are safe because the module description
 * `<requires>` both of them with no way to opt out, so selecting this module
 * guarantees their headers are present. Everything else is per-model and
 * reaches the bindings table as an opaque pointer instead.
 */
#include "r_ioport_api.h"
#include "r_timer_api.h"

/*
 * The generated configuration, from this module's <config> element. Lands in
 * ra_cfg/fsp_cfg/, which every FSP project has on its include path.
 */
#include "rm_pictorus_app_cfg.h"

FSP_HEADER

/* 
 * The Pictorus runtime instance, opaque on both sides of the seam. Declared
 * here rather than with the other Rust entry points below because the control
 * block holds one. 
 */
typedef struct st_AppInterface AppInterface;

/*
 * The peripherals a model is wired to.
 * Order within each array is the model's own I/O order, fixed at generation
 * time and matched by index on the Rust side.
 */
typedef struct st_rm_pictorus_bindings
{
    /* IOPORT is a singleton and it is already open by the time any of
     * this runs, from R_BSP_WarmStart(BSP_WARM_START_POST_C). */
    ioport_instance_t const * p_ioport;

    /* Timer that both paces the model and measures elapsed time.
     *
     * The timer instance must have an interrupt priority set in the
     * configurator. A blank priority allocates no vector, so the callback never
     * fires -- see the note in rm_pictorus_app.c. */
    timer_instance_t const * p_time_base;

    /* Pin identities in (port << 8) | pin form, one per GPIO block in the
     * model. Composed by the MDF from a port/pin `<option>` pair into the
     * BSP_IO_PORT_xx_PIN_yy macro, so an impossible selection is an undefined
     * identifier and fails to compile rather than misbehaving. */
    bsp_io_port_pin_t const * p_gpio_pins;
    uint8_t                   gpio_pin_count;

    /* Timers driving PWM outputs, one per PWM block. Typed, because
     * r_timer_api.h is guaranteed by the time-base requirement. Distinct from
     * p_time_base, which paces the model and is never a PWM output. */
    timer_instance_t const * const * pp_pwm;
    uint8_t                          pwm_count;

    /*
     * Peripherals beyond GPIO and PWM are not represented yet.
     *
     * Adding a field is an ABI change, and a silent one -- see the "What must
     * stay true" note above. Append rather than insert, and refresh
     * bindings/seam.rs in the same commit so the diff is visible.
     */
} rm_pictorus_bindings_t;

/* Configuration, generated into ra_gen/hal_data.c from the module's
 * `<declarations>`. */
typedef struct st_rm_pictorus_cfg
{
    rm_pictorus_bindings_t const * p_bindings;

    /* Heap handed to the Rust allocator.
     *
     * Sized in C rather than in Rust because the size is a GUI property, and a
     * GUI property reaches code as a `#define` in a generated `*_cfg.h`. Rust
     * cannot size a static array from a header it never compiles against, so C
     * declares the buffer and passes it over. Set BSP_CFG_HEAP_BYTES to 0 so
     * that FSP does not also reserve a `g_heap` nobody uses. 
     */
    void * p_heap;
    size_t heap_bytes;

    /* Run one time step per time-base period, rather than as fast as the loop
     * allows.
     *
     * The tick rate is the time base timer's own period, set on that timer 
     * in the configurator.
     */
    bool paced;
} rm_pictorus_cfg_t;

/* Allocated by the user, zero-initialised in .bss, populated by open(). Opaque
 * in the API signatures, as every FSP `<iface>_ctrl_t` is; the concrete type
 * below is what the user actually declares, exactly as `sci_uart_ctrl_t` is
 * opaque while `sci_uart_instance_ctrl_t` is not. */
typedef void rm_pictorus_ctrl_t;

/* Written into ctrl->open by open() and cleared by close(). An arbitrary
 * non-zero cookie, so that a call on a .bss-zeroed or already-closed control
 * block is rejected rather than acted on. "PICT" in ASCII. */
#define RM_PICTORUS_OPEN    (0x50494354U)

typedef struct st_rm_pictorus_instance_ctrl
{
    uint32_t                  open;
    rm_pictorus_cfg_t const * p_cfg;
    AppInterface            * p_app;

    /* Which model build this is, from pictorus_rt_build_id(). Held so a
     * debugger attached to a running board can answer the question. */
    char const * p_build_id;

    /* Set by the time-base callback, cleared by the loop. Not a counter of work
     * to do: a tick that arrives while the previous one is still running is a
     * missed deadline, not a queued item, and running two ticks back to back to
     * "catch up" would make an overrun worse. */
    volatile bool     tick_pending;
    volatile uint32_t missed_ticks;

    /* Software extension of the time base. The hardware counter is 16 or 32
     * bits and wraps every period; this counts the wraps. Written only by the
     * callback, read by the loop. */
    volatile uint64_t elapsed_periods;

    /* Cached from timer_api_t::infoGet at open(), because neither can change
     * without the timer being reconfigured, which only the configurator does. */
    uint32_t timer_clock_hz;
    uint32_t timer_period_counts;
} rm_pictorus_instance_ctrl_t;

typedef struct st_rm_pictorus_api
{
    /* Check the ABI, hand over the heap, publish the peripheral table, and
     * construct the model. Must run after FSP has brought up clocks and pins,
     * which it has by the time hal_entry() is reached. */
    fsp_err_t (* open)(rm_pictorus_ctrl_t * const p_ctrl, rm_pictorus_cfg_t const * const p_cfg);

    /* Run the model for one time step. `app_time_s` is measured elapsed time */
    fsp_err_t (* update)(rm_pictorus_ctrl_t * const p_ctrl, double app_time_s);

    /* Run until stopped, one step per time-base period when cfg->paced.
     * Does not return. The model
     * executes from this loop rather than from the timer ISR: it allocates, may
     * write telemetry, and its duration is a function of the user's block
     * diagram, so running it at interrupt priority would make every FSP
     * driver's latency depend on the model. */
    fsp_err_t (* run)(rm_pictorus_ctrl_t * const p_ctrl);

    fsp_err_t (* close)(rm_pictorus_ctrl_t * const p_ctrl);
} rm_pictorus_api_t;

typedef struct st_rm_pictorus_instance
{
    rm_pictorus_ctrl_t      * p_ctrl;
    rm_pictorus_cfg_t const * p_cfg;
    rm_pictorus_api_t const * p_api;
} rm_pictorus_instance_t;

/* The C-side dispatch table, defined in rm_pictorus_app.c. The generated
 * hal_data.c points its rm_pictorus_instance_t at this.*/
extern const rm_pictorus_api_t g_rm_pictorus_on_rm_pictorus;

/* The Rust allocator's arena, defined in rm_pictorus_app.c and sized from the
 * generated rm_pictorus_app_cfg.h. Declared here so that the module
 * description's <declarations> can point rm_pictorus_cfg_t::p_heap at it, which
 * keeps the size in exactly one place: the GUI property. */
extern uint8_t g_rm_pictorus_heap[];

/**********************************************************************************************************************
 * Rust entry points
 *
 * Defined in libpictorus_app.a. Every one of these is called from
 * rm_pictorus_app.c, so each is a genuine undefined symbol in the objects the
 * user's project compiles.
 *********************************************************************************************************************/

/* Identifies the model build the library was generated from. Points at static
 * storage inside the library; never freed.  */
char const * pictorus_rt_build_id(void);

/* Hand the Rust allocator its heap arena. Call once, before app_interface_new().
 * `p_base` must remain valid for the life of the program. */
void pictorus_rt_heap_init(void * p_base, size_t bytes);

/* Publish the peripheral table. Call once, before app_interface_new(); the
 * model's I/O wrappers are constructed from it. `p_bindings` and everything it
 * points at must remain valid for the life of the program, which the generated
 * hal_data.c satisfies by making them const globals. */
void pictorus_rt_bind(rm_pictorus_bindings_t const * p_bindings);

/* The Pictorus C-ABI library interface, emitted by codegen for LibType.STATIC.
 *
 * The two-argument `update` is the whole signature: a third `AppDataInput *`
 * parameter appears only when the model declares C-binding I/O structs, so
 * models targeting FSP must use peripheral-mediated I/O only. That restriction
 * is what lets one model-agnostic C shim serve every model. */
AppInterface * app_interface_new(void);
void           app_interface_update(AppInterface * p_app, double app_time_s);
void           app_interface_free(AppInterface * p_app);

FSP_FOOTER

#endif                                 /* RM_PICTORUS_APP_H */
