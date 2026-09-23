/*
 * rm_pictorus_app.c -- the FSP-side half of the Pictorus seam.
 *
 * Shipped in the generated pack as source, so the user's e2 studio project
 * compiles it against its own BSP. 
 */

#include "rm_pictorus_app.h"

/* Generated from the module description's <config> element into
 * ra_cfg/fsp_cfg/. Supplies RM_PICTORUS_APP_CFG_HEAP_BYTES. */
#include "rm_pictorus_app_cfg.h"

/**********************************************************************************************************************
 * The heap
 *********************************************************************************************************************/

/*
 * Set BSP_CFG_HEAP_BYTES to 0 in the BSP properties. Otherwise FSP reserves a
 * `g_heap` for newlib's sbrk that nothing here uses, and the two arenas compete
 * for the same RAM.
 *
 * Aligned to 8 rather than left to the compiler: the Rust allocator hands out
 * blocks from this buffer for types whose alignment it knows and this file does
 * not, and 8 covers every alignment a Cortex-M scalar or pointer can ask for.
 * The module description constrains the property to at least 256 bytes, so the
 * array is never zero-length.
 */
uint8_t g_rm_pictorus_heap[RM_PICTORUS_APP_CFG_HEAP_BYTES] BSP_ALIGN_VARIABLE(8);

/**********************************************************************************************************************
 * Forward declarations
 *********************************************************************************************************************/

static fsp_err_t rm_pictorus_open(rm_pictorus_ctrl_t * const p_ctrl, rm_pictorus_cfg_t const * const p_cfg);
static fsp_err_t rm_pictorus_update(rm_pictorus_ctrl_t * const p_ctrl, double app_time_s);
static fsp_err_t rm_pictorus_run(rm_pictorus_ctrl_t * const p_ctrl);
static fsp_err_t rm_pictorus_close(rm_pictorus_ctrl_t * const p_ctrl);

void rm_pictorus_timer_callback(timer_callback_args_t * p_args);

/*
 * The module vtable, following the same four-type convention as every FSP
 * driver so that the module reads as native in the Stacks tab and in Developer
 * Assistance snippets.
 */
const rm_pictorus_api_t g_rm_pictorus_on_rm_pictorus =
{
    .open   = rm_pictorus_open,
    .update = rm_pictorus_update,
    .run    = rm_pictorus_run,
    .close  = rm_pictorus_close,
};

/**********************************************************************************************************************
 * Time
 *********************************************************************************************************************/

/*
 * Elapsed seconds since open(), from the time-base timer.
 *
 * If the timer instance has no interrupt priority set in the configurator, no
 * vector is allocated, the callback never fires, `elapsed_periods` stays at
 * zero and this saturates at one period. That is the single most likely
 * misconfiguration for this module and it has no diagnostic from here, which is
 * why it is called out in the Developer Assistance text.
 */
static double rm_pictorus_time_seconds (rm_pictorus_instance_ctrl_t * const p_ctrl)
{
    timer_instance_t const * p_timer = p_ctrl->p_cfg->p_bindings->p_time_base;
    timer_status_t           status  = {0};

    /* elapsed_periods is 64-bit and the core is 32-bit, so a plain read is two
     * loads and the callback can land between them. Use critical section to 
     * guard against an ISR occuring between reads. */
    FSP_CRITICAL_SECTION_DEFINE;

    uint64_t periods;
    FSP_CRITICAL_SECTION_ENTER;
    periods = p_ctrl->elapsed_periods;
    FSP_CRITICAL_SECTION_EXIT;

    for (uint32_t attempt = 0; attempt < 2U; attempt++)
    {
        if (FSP_SUCCESS != p_timer->p_api->statusGet(p_timer->p_ctrl, &status))
        {
            status.counter = 0U;
            break;
        }

        uint64_t recheck;
        FSP_CRITICAL_SECTION_ENTER;
        recheck = p_ctrl->elapsed_periods;
        FSP_CRITICAL_SECTION_EXIT;

        if (recheck == periods)
        {
            break;
        }
        periods = recheck;
    }

    uint32_t elapsed_in_period = status.counter;
    if (TIMER_DIRECTION_DOWN == p_ctrl->timer_direction)
    {
        elapsed_in_period = p_ctrl->timer_period_counts - status.counter;
    }

    uint64_t counts = (periods * (uint64_t) p_ctrl->timer_period_counts) + (uint64_t) elapsed_in_period;

    return (double) counts / (double) p_ctrl->timer_clock_hz;
}

/*
 * Time-base expiry. Runs in ISR context and does the least possible.
 *
 * `p_context` is the control block, established by callbackSet in open(). The
 * timer's own configured callback, if the user set one, is displaced for the
 * duration -- that is what callbackSet is for, and a timer dedicated to this
 * module has no other consumer.
 */
void rm_pictorus_timer_callback (timer_callback_args_t * p_args)
{
    if ((NULL == p_args) || (NULL == p_args->p_context))
    {
        return;
    }

    rm_pictorus_instance_ctrl_t * p_ctrl = (rm_pictorus_instance_ctrl_t *) p_args->p_context;

    p_ctrl->elapsed_periods++;

    /* A tick that arrives while the previous one is still running is a missed
     * deadline. Counting it rather than queueing it keeps the loop from trying
     * to catch up, which would only lengthen the overrun. */
    if (p_ctrl->tick_pending)
    {
        p_ctrl->missed_ticks++;
    }

    p_ctrl->tick_pending = true;
}

/**********************************************************************************************************************
 * API
 *********************************************************************************************************************/

static fsp_err_t rm_pictorus_open (rm_pictorus_ctrl_t * const p_api_ctrl, rm_pictorus_cfg_t const * const p_cfg)
{
    rm_pictorus_instance_ctrl_t * p_ctrl = (rm_pictorus_instance_ctrl_t *) p_api_ctrl;

    FSP_ASSERT(NULL != p_ctrl);
    FSP_ASSERT(NULL != p_cfg);
    FSP_ASSERT(NULL != p_cfg->p_bindings);
    FSP_ASSERT(NULL != p_cfg->p_bindings->p_time_base);
    FSP_ERROR_RETURN(RM_PICTORUS_OPEN != p_ctrl->open, FSP_ERR_ALREADY_OPEN);

    p_ctrl->p_cfg           = p_cfg;
    p_ctrl->p_build_id      = pictorus_rt_build_id();
    p_ctrl->tick_pending    = false;
    p_ctrl->missed_ticks    = 0U;
    p_ctrl->elapsed_periods = 0U;

    /* Timer first: the model's constructors may read the clock, and a Pictorus
     * block that samples time before the time base is characterised would get
     * a division by zero. */
    timer_instance_t const * p_timer = p_cfg->p_bindings->p_time_base;
    timer_info_t             info    = {0};

    /*
     * The time base must raise an interrupt, in both paced and free-running
     * mode. Its callback is the only thing that increments `elapsed_periods`,
     * and without that the counter read below is just the offset within one
     * period: model time would run to the end of a single timer period and stop.
     *
     * Checked here so that it is a clear failure at startup instead.
     */
    FSP_ERROR_RETURN(FSP_INVALID_VECTOR != p_timer->p_cfg->cycle_end_irq,
                     FSP_ERR_IRQ_BSP_DISABLED);

    fsp_err_t err = p_timer->p_api->open(p_timer->p_ctrl, p_timer->p_cfg);
    FSP_ERROR_RETURN(FSP_SUCCESS == err || FSP_ERR_ALREADY_OPEN == err, err);

    err = p_timer->p_api->infoGet(p_timer->p_ctrl, &info);
    FSP_ERROR_RETURN(FSP_SUCCESS == err, err);
    FSP_ERROR_RETURN(0U != info.clock_frequency, FSP_ERR_INVALID_HW_CONDITION);

    p_ctrl->timer_clock_hz      = info.clock_frequency;
    p_ctrl->timer_period_counts = info.period_counts;
    p_ctrl->timer_direction     = info.count_direction;

    err = p_timer->p_api->callbackSet(p_timer->p_ctrl, rm_pictorus_timer_callback, p_ctrl, NULL);
    FSP_ERROR_RETURN(FSP_SUCCESS == err, err);

    /* Heap before the model: app_interface_new() allocates. */
    pictorus_rt_heap_init(p_cfg->p_heap, p_cfg->heap_bytes);

    /* Peripherals before the model, for the same reason: the model's I/O
     * wrappers are constructed inside app_interface_new(). */
    pictorus_rt_bind(p_cfg->p_bindings);

    p_ctrl->p_app = app_interface_new();
    FSP_ERROR_RETURN(NULL != p_ctrl->p_app, FSP_ERR_OUT_OF_MEMORY);

    err = p_timer->p_api->start(p_timer->p_ctrl);
    FSP_ERROR_RETURN(FSP_SUCCESS == err, err);

    p_ctrl->open = RM_PICTORUS_OPEN;

    return FSP_SUCCESS;
}

/* One time step, at the measured time. */
static fsp_err_t rm_pictorus_update (rm_pictorus_ctrl_t * const p_api_ctrl, double app_time_s)
{
    rm_pictorus_instance_ctrl_t * p_ctrl = (rm_pictorus_instance_ctrl_t *) p_api_ctrl;

    FSP_ASSERT(NULL != p_ctrl);
    FSP_ERROR_RETURN(RM_PICTORUS_OPEN == p_ctrl->open, FSP_ERR_NOT_OPEN);

    app_interface_update(p_ctrl->p_app, app_time_s);

    return FSP_SUCCESS;
}

/*
 * Run the model indefinitely.
 *
 * Two modes. Paced waits for the time base and runs one step per timer period,
 * so the tick rate is the timer's configured period. Free running executes as
 * fast as the loop allows. Either way the time handed to the model is measured, 
 * so the two behave identically apart from their step size.
 */
static fsp_err_t rm_pictorus_run (rm_pictorus_ctrl_t * const p_api_ctrl)
{
    rm_pictorus_instance_ctrl_t * p_ctrl = (rm_pictorus_instance_ctrl_t *) p_api_ctrl;

    FSP_ASSERT(NULL != p_ctrl);
    FSP_ERROR_RETURN(RM_PICTORUS_OPEN == p_ctrl->open, FSP_ERR_NOT_OPEN);

    bool paced = p_ctrl->p_cfg->paced;

    while (RM_PICTORUS_OPEN == p_ctrl->open)
    {
        if (paced)
        {
            /* Test and clear together, under a critical section.*/
            bool run_step = false;
            FSP_CRITICAL_SECTION_DEFINE;
            FSP_CRITICAL_SECTION_ENTER;
            if (p_ctrl->tick_pending)
            {
                p_ctrl->tick_pending = false;
                run_step             = true;
            }
            FSP_CRITICAL_SECTION_EXIT;

            if (!run_step)
            {
                continue;
            }
        }

        app_interface_update(p_ctrl->p_app, rm_pictorus_time_seconds(p_ctrl));
    }

    return FSP_SUCCESS;
}

static fsp_err_t rm_pictorus_close (rm_pictorus_ctrl_t * const p_api_ctrl)
{
    rm_pictorus_instance_ctrl_t * p_ctrl = (rm_pictorus_instance_ctrl_t *) p_api_ctrl;

    FSP_ASSERT(NULL != p_ctrl);
    FSP_ERROR_RETURN(RM_PICTORUS_OPEN == p_ctrl->open, FSP_ERR_NOT_OPEN);

    /* Cleared first, so that a time-base interrupt arriving during teardown
     * sees a closed instance and a run() loop already on its way out. */
    p_ctrl->open = 0U;

    timer_instance_t const * p_timer = p_ctrl->p_cfg->p_bindings->p_time_base;
    p_timer->p_api->stop(p_timer->p_ctrl);

    app_interface_free(p_ctrl->p_app);
    p_ctrl->p_app = NULL;

    return FSP_SUCCESS;
}
