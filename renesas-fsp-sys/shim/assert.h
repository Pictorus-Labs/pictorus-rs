/*
 * Freestanding stub for <assert.h>.
 *
 * Bindings are generated with -nostdlibinc, so the host's libc headers cannot
 * leak host-sized types into a target-ABI binding. clang still supplies
 * stdint.h, stddef.h and stdbool.h from its own resource directory, leaving
 * <assert.h> -- included by fsp_common_api.h -- as the only gap. Stubbing it is
 * safe because no interface-layer declaration uses `assert`; the driver sources
 * that do are compiled by the user's project against their own libc.
 */

#ifndef FSP_SYS_STUB_ASSERT_H
#define FSP_SYS_STUB_ASSERT_H

#undef assert
#define assert(expr)    ((void) 0)

#endif
