/*
 * app_interface.h -- hand-written stand-in for the header Pictorus generates
 * per app. It has the same shape as a generated one so that the C tests
 * exercise the same calls an integrator makes.
 */
#ifndef APP_INTERFACE_H
#define APP_INTERFACE_H

#include <stddef.h>
#include <stdint.h>

#include "pictorus_ffi.h"

#ifdef __cplusplus
extern "C" {
#endif

#define APP_INTERFACE_FUNDAMENTAL_TIMESTEP_US 10000

typedef struct AppInterface AppInterface;

typedef struct {
    double setpoint;
} AppInputs;

typedef struct {
    double output;
    uint64_t steps;
} AppOutputs;

size_t app_interface_size(void);
size_t app_interface_align(void);
uint64_t app_interface_fundamental_timestep_us(void);

/* mem must be app_interface_size() bytes, aligned to app_interface_align(). */
pictorus_status_t app_interface_init(void *mem, AppInterface **out);
pictorus_status_t app_interface_step(AppInterface *app, uint64_t app_time_us,
                                     const AppInputs *inputs, AppOutputs *outputs);
void app_interface_deinit(AppInterface *app);

/* Test hook: number of instances initialized and not yet deinitialized. */
size_t fixture_live_instances(void);

#ifdef __cplusplus
}
#endif

#endif /* APP_INTERFACE_H */
