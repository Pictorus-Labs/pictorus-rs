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

/* One field per named peripheral. */
typedef struct {
    pictorus_gpio_in_t pwr_btn;
    pictorus_gpio_out_t status_led;
    pictorus_pwm_t motor_pwm;
    pictorus_adc_t vbat;
    pictorus_dac_t analog_out;
} AppPeripherals;

typedef struct {
    pictorus_peripheral_status_t pwr_btn;
    pictorus_peripheral_status_t status_led;
    pictorus_peripheral_status_t motor_pwm;
    pictorus_peripheral_status_t vbat;
    pictorus_peripheral_status_t analog_out;
} AppPeripheralStatus;

typedef struct {
    double setpoint;
} AppInputs;

typedef struct {
    double output;
    uint64_t steps;
    /* The model's view of the inputs it read this tick. */
    double button;
    double vbat_a;
    double vbat_b;
} AppOutputs;

size_t app_interface_size(void);
size_t app_interface_align(void);
uint64_t app_interface_fundamental_timestep_us(void);

/*
 * mem must be app_interface_size() bytes, aligned to app_interface_align().
 * peripherals is copied, so the caller's copy may be discarded afterwards.
 */
pictorus_status_t app_interface_init(void *mem, const AppPeripherals *peripherals,
                                     AppInterface **out);

/*
 * Returns PICTORUS_ERR_PERIPHERAL if any peripheral operation failed during
 * the tick. The model still ran and outputs are valid.
 */
pictorus_status_t app_interface_step(AppInterface *app, uint64_t app_time_us,
                                     const AppInputs *inputs, AppOutputs *outputs);

pictorus_status_t app_interface_peripheral_status(const AppInterface *app,
                                                  AppPeripheralStatus *out);

void app_interface_deinit(AppInterface *app);

/* Test hook: number of instances initialized and not yet deinitialized. */
size_t fixture_live_instances(void);

#ifdef __cplusplus
}
#endif

#endif /* APP_INTERFACE_H */
