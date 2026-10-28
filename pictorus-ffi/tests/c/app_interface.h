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
    pictorus_i2c_t imu_bus;
    pictorus_spi_t flash;
    pictorus_serial_t console;
    pictorus_can_t can0;
    pictorus_udp_t telemetry;
} AppPeripherals;

typedef struct {
    pictorus_peripheral_status_t pwr_btn;
    pictorus_peripheral_status_t status_led;
    pictorus_peripheral_status_t motor_pwm;
    pictorus_peripheral_status_t vbat;
    pictorus_peripheral_status_t analog_out;
    pictorus_peripheral_status_t imu_bus;
    pictorus_peripheral_status_t flash;
    pictorus_peripheral_status_t console;
    pictorus_peripheral_status_t can0;
    pictorus_peripheral_status_t telemetry;
} AppPeripheralStatus;

/* Only the peripherals with app level settings have one. */
typedef struct {
    pictorus_i2c_config_t imu_bus;
    pictorus_spi_config_t flash;
    pictorus_serial_config_t console;
    pictorus_can_config_t can0;
    pictorus_udp_config_t telemetry;
} AppPeripheralConfig;

static const AppPeripheralConfig APP_PERIPHERAL_CONFIG = {
    .imu_bus = {.frequency_hz = 400000},
    .flash = {.frequency_hz = 1000000,
              .mode = 3,
              .bit_order = PICTORUS_SPI_MSB_FIRST,
              .bits_per_transfer = 8},
    .console = {.baud_rate = 115200},
    .can0 = {.bitrate = 500000},
    .telemetry = {.address = "0.0.0.0:9001"},
};

typedef struct {
    double setpoint;
} AppInputs;

/* What a byte-oriented input block produced in a tick. */
typedef struct {
    uint32_t len;
    uint8_t first;
} AppRxSummary;

typedef struct {
    double output;
    uint64_t steps;
    /* The model's view of the inputs it read this tick. */
    double button;
    double vbat_a;
    double vbat_b;
    AppRxSummary imu;
    AppRxSummary flash_a;
    AppRxSummary flash_b;
    AppRxSummary console_a;
    AppRxSummary console_b;
    AppRxSummary can;
    AppRxSummary telemetry;
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

/* Test hooks. */
size_t mock_live_instances(void);
/* sizeof the types that cross the boundary, as Rust lays them out. */
size_t mock_can_frame_size(void);
size_t mock_peripheral_status_size(void);
size_t mock_peripherals_size(void);
size_t mock_outputs_size(void);

#ifdef __cplusplus
}
#endif

#endif /* APP_INTERFACE_H */
