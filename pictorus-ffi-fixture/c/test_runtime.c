/* Exercises the runtime and the app lifecycle the way an integrator would. */
#include <stdio.h>
#include <stdlib.h>

#include "app_interface.h"
#include "pictorus_ffi.h"

/* Absolute symbols exported by the library, see size.rs in pictorus-ffi. */
extern char PICTORUS_APP_INTERFACE_SIZE;
extern char PICTORUS_APP_INTERFACE_ALIGN;

#define CHECK(cond)                                                          \
    do {                                                                     \
        if (!(cond)) {                                                       \
            fprintf(stderr, "%s:%d: check failed: %s\n", __FILE__, __LINE__, \
                    #cond);                                                  \
            exit(1);                                                         \
        }                                                                    \
    } while (0)

static void *alloc_instance(void) {
    void *mem = aligned_alloc(app_interface_align(), app_interface_size());
    CHECK(mem != NULL);
    return mem;
}

int main(void) {
    CHECK(pictorus_rt_abi_version() == PICTORUS_FFI_ABI_VERSION);
    CHECK(app_interface_fundamental_timestep_us() ==
          APP_INTERFACE_FUNDAMENTAL_TIMESTEP_US);

    /* The link-time size symbols agree with the runtime functions. */
    CHECK((size_t)&PICTORUS_APP_INTERFACE_SIZE == app_interface_size());
    CHECK((size_t)&PICTORUS_APP_INTERFACE_ALIGN == app_interface_align());

    /* Bad config is rejected without consuming the one-shot init. */
    pictorus_rt_config_t bad = {.heap = NULL, .heap_bytes = 16};
    CHECK(pictorus_rt_init(&bad) == PICTORUS_ERR_INVALID_ARG);

    CHECK(pictorus_rt_init(NULL) == PICTORUS_OK);
    CHECK(pictorus_rt_init(NULL) == PICTORUS_ERR_ALREADY_INIT);

    /* Invalid arguments. */
    AppInterface *app = NULL;
    CHECK(app_interface_init(NULL, &app) == PICTORUS_ERR_INVALID_ARG);

    /* Two independent instances of the same app, in caller-provided memory. */
    void *mem_a = alloc_instance();
    void *mem_b = alloc_instance();
    AppInterface *a = NULL;
    AppInterface *b = NULL;
    CHECK(app_interface_init(mem_a, &a) == PICTORUS_OK);
    CHECK(app_interface_init(mem_b, &b) == PICTORUS_OK);
    CHECK(a == mem_a && b == mem_b);
    CHECK(fixture_live_instances() == 2);

    AppInputs in_a = {.setpoint = 1.5};
    AppInputs in_b = {.setpoint = -4.0};
    AppOutputs out_a = {0};
    AppOutputs out_b = {0};

    for (uint64_t i = 1; i <= 3; i++) {
        CHECK(app_interface_step(a, i * 10000, &in_a, &out_a) == PICTORUS_OK);
        CHECK(out_a.steps == i);
    }
    CHECK(out_a.output == 3.0);

    /* b has not been stepped, so a's steps must not have leaked into it. */
    CHECK(app_interface_step(b, 10000, &in_b, &out_b) == PICTORUS_OK);
    CHECK(out_b.steps == 1);
    CHECK(out_b.output == -8.0);

    CHECK(app_interface_step(a, 0, NULL, &out_a) == PICTORUS_ERR_INVALID_ARG);

    app_interface_deinit(a);
    CHECK(fixture_live_instances() == 1);
    app_interface_deinit(b);
    CHECK(fixture_live_instances() == 0);

    free(mem_a);
    free(mem_b);
    printf("ok\n");
    return 0;
}
