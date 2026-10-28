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

/*
 * I2C bus. Addresses are the 7 bit address, not shifted. Both operations
 * perform a complete bus transaction, and reads and writes are not cached, since
 * several devices can share one bus.
 *
 * write_read writes wr, issues a repeated start, and reads rd_len bytes.
 */
typedef struct {
    void *ctx;
    pictorus_status_t (*write_read)(void *ctx, uint8_t addr, const uint8_t *wr,
                                    size_t wr_len, uint8_t *rd, size_t rd_len);
    pictorus_status_t (*write)(void *ctx, uint8_t addr, const uint8_t *wr,
                               size_t wr_len);
} pictorus_i2c_t;

/*
 * SPI device. Chip select belongs to this side: each call is one complete,
 * chip-select framed transaction. read is called at most once per tick. The
 * library only ever reads or writes in a call, never both, so what read clocks
 * out on MOSI while it receives is this side's choice.
 */
typedef struct {
    void *ctx;
    pictorus_status_t (*read)(void *ctx, uint8_t *rx, size_t len);
    pictorus_status_t (*write)(void *ctx, const uint8_t *tx, size_t len);
} pictorus_spi_t;

/*
 * Serial port. read must not block. Write to buf the bytes received since the
 * last call, at most cap of them, and set *len to how many. Return PICTORUS_EMPTY
 * if there are none. Bytes left unread should be returned by a later call. read is
 * called at most once per tick.
 */
typedef struct {
    void *ctx;
    pictorus_status_t (*read)(void *ctx, uint8_t *buf, size_t cap, size_t *len);
    pictorus_status_t (*write)(void *ctx, const uint8_t *buf, size_t len);
} pictorus_serial_t;

/* A classic CAN data frame. */
typedef struct {
    /* 11 bit identifier, or 29 bit if extended is true. */
    uint32_t id;
    bool extended;
    /* Number of valid bytes in data, at most 8. */
    uint8_t dlc;
    uint8_t data[8];
} pictorus_can_frame_t;

/*
 * CAN bus. receive must not block. Fill *frame and return PICTORUS_OK, or return
 * PICTORUS_EMPTY once the receive queue is drained. Remote frames should not be
 * reported. A tick takes at most 64 frames from receive, and the rest are taken on
 * later ticks.
 */
typedef struct {
    void *ctx;
    pictorus_status_t (*receive)(void *ctx, pictorus_can_frame_t *frame);
    pictorus_status_t (*transmit)(void *ctx, const pictorus_can_frame_t *frame);
} pictorus_can_t;

/*
 * UDP socket, backed by whatever network stack the target has. Binding to the
 * address in the app's peripheral configuration is this side's job.
 *
 * recv must not block. It works as the serial read does, for one datagram: write
 * it to buf, at most cap bytes, set *len, and return PICTORUS_EMPTY if there is
 * none. It is called until it reports empty, and the most recent datagram of the
 * tick is the one the model sees.
 *
 * destination is the NUL-terminated "address:port" from the transmit block.
 */
typedef struct {
    void *ctx;
    pictorus_status_t (*recv)(void *ctx, uint8_t *buf, size_t cap, size_t *len);
    pictorus_status_t (*send)(void *ctx, const char *destination, const uint8_t *buf,
                              size_t len);
} pictorus_udp_t;

/* Health of one peripheral, reported per named peripheral by the app. */
typedef struct {
    /* The most recent failure, or PICTORUS_OK if there has never been one. */
    pictorus_status_t last_error;
    /* Failed operations since init. Saturates at UINT32_MAX. */
    uint32_t error_count;
} pictorus_peripheral_status_t;

/* ---- Peripheral configuration --------------------------------------------
 *
 * The settings a model chooses for a peripheral, which this side applies when it
 * initializes the hardware. Peripheral kinds with no such settings (GPIO, PWM, ADC
 * and DAC) have no struct. The generated app header has a static const instance
 * for each peripheral that does.
 */

typedef struct {
    uint32_t frequency_hz;
} pictorus_i2c_config_t;

/* Values for pictorus_spi_config_t.bit_order. */
#define PICTORUS_SPI_MSB_FIRST 0
#define PICTORUS_SPI_LSB_FIRST 1

typedef struct {
    uint32_t frequency_hz;
    /* Clock polarity and phase, 0 to 3. */
    uint8_t mode;
    /* PICTORUS_SPI_MSB_FIRST or PICTORUS_SPI_LSB_FIRST. */
    uint8_t bit_order;
    /* 8 or 16. */
    uint8_t bits_per_transfer;
} pictorus_spi_config_t;

typedef struct {
    uint32_t baud_rate;
} pictorus_serial_config_t;

typedef struct {
    uint32_t bitrate;
} pictorus_can_config_t;

typedef struct {
    /* The "address:port" to bind to, as a NUL-terminated string. */
    const char *address;
} pictorus_udp_config_t;

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
     * Optional pair that makes the library's shared state safe when app
     * instances are used from more than one task or interrupt context. The
     * library holds a critical section around each allocation and, on targets
     * with no atomic read-modify-write instructions such as Cortex-M0, around
     * its own atomic operations.
     *
     * enter must prevent any other context from running library code that
     * takes a critical section until the matching exit, and returns an opaque
     * state. exit is passed the state its matching enter returned. Sections can
     * nest, so the state should capture what is needed to put things back as
     * they were, for example the previous interrupt mask.
     *
     * What these functions do is up to the integrator. The library only
     * promises to call them around code that is unsafe to run from two contexts
     * at once. If they mask interrupts, interrupts stay masked for as long as
     * that code runs, which for an allocation is not constant time.
     *
     * If either is NULL the library is unprotected, and all calls into it must
     * come from a single context. Ignored on targets with an operating system.
     */
    uint32_t (*critical_section_enter)(void);
    void (*critical_section_exit)(uint32_t state);
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
