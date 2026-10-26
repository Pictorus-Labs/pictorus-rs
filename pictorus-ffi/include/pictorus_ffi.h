/*
 * pictorus_ffi.h -- runtime interface of the Pictorus C FFI.
 *
 * This header is the same for every app. Per-app declarations live in the
 * generated app_interface.h.
 */
#ifndef PICTORUS_FFI_H
#define PICTORUS_FFI_H

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
