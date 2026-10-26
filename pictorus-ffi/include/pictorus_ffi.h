/*
 * pictorus_ffi.h -- runtime interface of the Pictorus C FFI.
 *
 * This header is the same for every app. Per-app declarations live in the
 * generated app_interface.h.
 */
#ifndef PICTORUS_FFI_H
#define PICTORUS_FFI_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Bumped on any incompatible change to this header. */
#define PICTORUS_FFI_ABI_VERSION 1

/* ---- Status codes ------------------------------------------------------- */

typedef int32_t pictorus_status_t;

#define PICTORUS_OK 0
/* No data available. Not an error. */
#define PICTORUS_EMPTY 1
#define PICTORUS_ERR_PERIPHERAL (-1)
#define PICTORUS_ERR_INVALID_ARG (-2)
#define PICTORUS_ERR_ALREADY_INIT (-3)

/* ---- Peripherals --------------------------------------------------------
 *
 * Each peripheral kind has a fixed handle type: a context pointer and the
 * functions that implement the kind's operations. The functions are plain "do
 * it now" operations shaped like vendor HAL calls. Anything the model expects
 * beyond that, such as reading an ADC at most once per tick, is handled by the
 * library.
 *
 * ctx identifies the instance, for example a HAL handle or a bus number. It is
 * passed back to every call and must remain valid until app_interface_deinit.
 * A function left NULL is treated as a failing operation.
 *
 * A function that fails should return a negative status. The library treats a
 * failure as "no new data": an input keeps its previous value and an output is
 * retried on the next tick. Failures are counted per peripheral, see
 * pictorus_peripheral_status_t.
 *
 * Buffers passed to a function belong to the library. Use them only during the
 * call and never keep the pointer.
 */

/* Digital input. Set *level to true when the pin is high. */
typedef struct {
    void *ctx;
    pictorus_status_t (*read)(void *ctx, bool *level);
} pictorus_gpio_in_t;

/* Digital output. */
typedef struct {
    void *ctx;
    pictorus_status_t (*write)(void *ctx, bool level);
} pictorus_gpio_out_t;

/*
 * Two channel PWM output, one handle per timer. Called only when the requested
 * values change. Duty cycles are in [0, 1]. The frequency is not negative and
 * may be zero.
 */
typedef struct {
    void *ctx;
    pictorus_status_t (*set)(void *ctx, double frequency_hz, double duty_ch1,
                             double duty_ch2);
} pictorus_pwm_t;

/* ADC input. Called at most once per tick. */
typedef struct {
    void *ctx;
    pictorus_status_t (*read)(void *ctx, uint16_t *sample);
} pictorus_adc_t;

/*
 * DAC output. values holds one entry per channel, in channel order, exactly as
 * the model produced them. Converting them to the hardware's units is up to
 * this function.
 */
typedef struct {
    void *ctx;
    pictorus_status_t (*write)(void *ctx, const double *values, size_t n);
} pictorus_dac_t;

/* Health of one peripheral, reported per named peripheral by the app. */
typedef struct {
    /* The most recent failure, or PICTORUS_OK if there has never been one. */
    pictorus_status_t last_error;
    /* Failed operations since init. Saturates at UINT32_MAX. */
    uint32_t error_count;
} pictorus_peripheral_status_t;

/* ---- Runtime ------------------------------------------------------------ */

typedef struct {
    /*
     * Memory the runtime allocator manages, shared by every app instance.
     * May be NULL (with heap_bytes == 0) if the app does not allocate.
     * Ignored on targets with an operating system.
     */
    void *heap;
    size_t heap_bytes;

    /*
     * Optional. Called with a UTF-8 description of an internal panic, which is
     * not NUL terminated and may be truncated. Must not return. If NULL, or if
     * it returns, the runtime halts. Ignored on targets with an operating system.
     */
    void (*on_panic)(const char *msg, size_t len);

    /*
     * Optional pair used to make the runtime allocator safe when app instances
     * are stepped from more than one task or interrupt context. enter must
     * prevent any other context from allocating until exit is called. If NULL
     * the allocator is unprotected, and all allocating calls into the library
     * must come from a single context. Ignored on targets with an operating
     * system.
     */
    void (*critical_section_enter)(void);
    void (*critical_section_exit)(void);
} pictorus_rt_config_t;

/*
 * Must be called exactly once, before any app instance is initialized.
 * config may be NULL, which is equivalent to an all-zero config.
 * Returns PICTORUS_ERR_INVALID_ARG if heap and heap_bytes disagree about
 * whether there is a heap, and PICTORUS_ERR_ALREADY_INIT on a repeat call.
 */
pictorus_status_t pictorus_rt_init(const pictorus_rt_config_t *config);

/*
 * Returns PICTORUS_FFI_ABI_VERSION as the library was built. Compare it
 * against the value from the header you compiled with.
 */
uint32_t pictorus_rt_abi_version(void);

#ifdef __cplusplus
}
#endif

#endif /* PICTORUS_FFI_H */
