/* Exercises the runtime, peripherals and app lifecycle the way an integrator would. */
#include <math.h>
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

/* ---- Fake board --------------------------------------------------------- */

typedef struct {
    bool level;
    pictorus_status_t result;
    int calls;
} fake_button_t;

typedef struct {
    bool last_level;
    pictorus_status_t result;
    int calls;
} fake_led_t;

typedef struct {
    double freq, duty1, duty2;
    pictorus_status_t result;
    int calls;
} fake_pwm_t;

typedef struct {
    uint16_t sample;
    pictorus_status_t result;
    int calls;
} fake_adc_t;

typedef struct {
    double values[4];
    size_t n;
    pictorus_status_t result;
    int calls;
} fake_dac_t;

typedef struct {
    fake_button_t button;
    fake_led_t led;
    fake_pwm_t pwm;
    fake_adc_t adc;
    fake_dac_t dac;
} fake_board_t;

static pictorus_status_t button_read(void *ctx, bool *level) {
    fake_button_t *f = ctx;
    f->calls++;
    if (f->result == PICTORUS_OK) *level = f->level;
    return f->result;
}

static pictorus_status_t led_write(void *ctx, bool level) {
    fake_led_t *f = ctx;
    f->calls++;
    f->last_level = level;
    return f->result;
}

static pictorus_status_t pwm_set(void *ctx, double freq, double d1, double d2) {
    fake_pwm_t *f = ctx;
    f->calls++;
    f->freq = freq;
    f->duty1 = d1;
    f->duty2 = d2;
    return f->result;
}

static pictorus_status_t adc_read(void *ctx, uint16_t *sample) {
    fake_adc_t *f = ctx;
    f->calls++;
    if (f->result == PICTORUS_OK) *sample = f->sample;
    return f->result;
}

static pictorus_status_t dac_write(void *ctx, const double *values, size_t n) {
    fake_dac_t *f = ctx;
    f->calls++;
    f->n = n;
    for (size_t i = 0; i < n && i < 4; i++) f->values[i] = values[i];
    return f->result;
}

static AppPeripherals peripherals_for(fake_board_t *board) {
    AppPeripherals p = {
        .pwr_btn = {.ctx = &board->button, .read = button_read},
        .status_led = {.ctx = &board->led, .write = led_write},
        .motor_pwm = {.ctx = &board->pwm, .set = pwm_set},
        .vbat = {.ctx = &board->adc, .read = adc_read},
        .analog_out = {.ctx = &board->dac, .write = dac_write},
    };
    return p;
}

static void *alloc_instance(void) {
    void *mem = aligned_alloc(app_interface_align(), app_interface_size());
    CHECK(mem != NULL);
    return mem;
}

static bool near(double a, double b) { return fabs(a - b) < 1e-9; }

/* ---- Tests -------------------------------------------------------------- */

static void test_runtime(void) {
    CHECK(pictorus_rt_abi_version() == PICTORUS_FFI_ABI_VERSION);

    /* The link-time size symbols agree with the runtime functions. */
    CHECK((size_t)&PICTORUS_APP_INTERFACE_SIZE == app_interface_size());
    CHECK((size_t)&PICTORUS_APP_INTERFACE_ALIGN == app_interface_align());
    CHECK(app_interface_fundamental_timestep_us() ==
          APP_INTERFACE_FUNDAMENTAL_TIMESTEP_US);

    /* Bad config is rejected without consuming the one-shot init. */
    pictorus_rt_config_t bad = {.heap = NULL, .heap_bytes = 16};
    CHECK(pictorus_rt_init(&bad) == PICTORUS_ERR_INVALID_ARG);

    CHECK(pictorus_rt_init(NULL) == PICTORUS_OK);
    CHECK(pictorus_rt_init(NULL) == PICTORUS_ERR_ALREADY_INIT);
}

static void test_init_validation(void) {
    fake_board_t board = {0};
    AppPeripherals p = peripherals_for(&board);
    AppInterface *app = NULL;
    void *mem = alloc_instance();

    CHECK(app_interface_init(NULL, &p, &app) == PICTORUS_ERR_INVALID_ARG);
    CHECK(app_interface_init(mem, NULL, &app) == PICTORUS_ERR_INVALID_ARG);
    CHECK(fixture_live_instances() == 0);
    free(mem);
}

static void test_peripherals_and_independent_instances(void) {
    fake_board_t board_a = {.adc = {.sample = 2048}};
    fake_board_t board_b = {.button = {.level = true}, .adc = {.sample = 100}};
    AppPeripherals pa = peripherals_for(&board_a);
    AppPeripherals pb = peripherals_for(&board_b);

    void *mem_a = alloc_instance();
    void *mem_b = alloc_instance();
    AppInterface *a = NULL;
    AppInterface *b = NULL;
    CHECK(app_interface_init(mem_a, &pa, &a) == PICTORUS_OK);
    CHECK(app_interface_init(mem_b, &pb, &b) == PICTORUS_OK);
    CHECK(a == mem_a && b == mem_b);
    CHECK(fixture_live_instances() == 2);

    /* The peripheral table is copied, so the caller's copy may go away. */
    pa = (AppPeripherals){0};
    pb = (AppPeripherals){0};

    AppInputs in_a = {.setpoint = 1.5};
    AppInputs in_b = {.setpoint = -4.0};
    AppOutputs out_a = {0};
    AppOutputs out_b = {0};

    /* First tick of a. */
    CHECK(app_interface_step(a, 10000, &in_a, &out_a) == PICTORUS_OK);
    CHECK(out_a.steps == 1);
    CHECK(out_a.output == 3.0);
    CHECK(out_a.button == 0.0 && board_a.led.last_level == false);

    /* Two blocks read the ADC, C is asked once and both see the same sample. */
    CHECK(board_a.adc.calls == 1);
    CHECK(out_a.vbat_a == 2048 && out_a.vbat_b == 2048);

    CHECK(board_a.pwm.calls == 1);
    CHECK(board_a.pwm.freq == 1000.0);
    CHECK(near(board_a.pwm.duty1, 2048.0 / 4095.0));
    CHECK(near(board_a.pwm.duty2, 1.0 - 2048.0 / 4095.0));

    CHECK(board_a.dac.calls == 1 && board_a.dac.n == 2);
    CHECK(board_a.dac.values[0] == 1.5 && board_a.dac.values[1] == 3.0);

    /* Second tick: the ADC is read again, but PWM is only touched on change. */
    CHECK(app_interface_step(a, 20000, &in_a, &out_a) == PICTORUS_OK);
    CHECK(board_a.adc.calls == 2);
    CHECK(board_a.pwm.calls == 1);

    board_a.adc.sample = 4095;
    CHECK(app_interface_step(a, 30000, &in_a, &out_a) == PICTORUS_OK);
    CHECK(board_a.pwm.calls == 2);
    CHECK(near(board_a.pwm.duty1, 1.0) && near(board_a.pwm.duty2, 0.0));
    CHECK(out_a.steps == 3);

    /* b has its own peripherals and its own state. */
    CHECK(app_interface_step(b, 10000, &in_b, &out_b) == PICTORUS_OK);
    CHECK(out_b.steps == 1);
    CHECK(out_b.output == -8.0);
    CHECK(out_b.button == 1.0 && board_b.led.last_level == true);
    CHECK(board_a.button.calls == 3 && board_b.button.calls == 1);

    /* Invalid step arguments. */
    CHECK(app_interface_step(a, 0, NULL, &out_a) == PICTORUS_ERR_INVALID_ARG);

    AppPeripheralStatus status;
    CHECK(app_interface_peripheral_status(a, &status) == PICTORUS_OK);
    CHECK(status.pwr_btn.error_count == 0 && status.vbat.error_count == 0);

    app_interface_deinit(a);
    CHECK(fixture_live_instances() == 1);
    app_interface_deinit(b);
    CHECK(fixture_live_instances() == 0);
    free(mem_a);
    free(mem_b);
}

static void test_failures_are_reported_but_do_not_change_the_model(void) {
    fake_board_t board = {.button = {.level = true}, .adc = {.sample = 800}};
    AppPeripherals p = peripherals_for(&board);
    void *mem = alloc_instance();
    AppInterface *app = NULL;
    CHECK(app_interface_init(mem, &p, &app) == PICTORUS_OK);

    AppInputs in = {.setpoint = 1.0};
    AppOutputs out = {0};
    CHECK(app_interface_step(app, 10000, &in, &out) == PICTORUS_OK);
    CHECK(out.button == 1.0 && out.vbat_a == 800);

    /* Inputs fail: the model keeps seeing the last good values. */
    board.button.result = PICTORUS_ERR_PERIPHERAL;
    board.button.level = false;
    board.adc.result = PICTORUS_ERR_PERIPHERAL;
    board.adc.sample = 1;
    CHECK(app_interface_step(app, 20000, &in, &out) == PICTORUS_ERR_PERIPHERAL);
    CHECK(out.button == 1.0 && out.vbat_a == 800 && out.vbat_b == 800);
    CHECK(out.output == 2.0 && out.steps == 2);

    /* The ADC failure was not retried within the tick. */
    CHECK(board.adc.calls == 2);

    AppPeripheralStatus status;
    CHECK(app_interface_peripheral_status(app, &status) == PICTORUS_OK);
    CHECK(status.pwr_btn.error_count == 1);
    CHECK(status.pwr_btn.last_error == PICTORUS_ERR_PERIPHERAL);
    CHECK(status.vbat.error_count == 1);
    CHECK(status.status_led.error_count == 0);
    CHECK(status.motor_pwm.error_count == 0);

    /* Recovery: values resume, and the history of failures is kept. */
    board.button.result = PICTORUS_OK;
    board.adc.result = PICTORUS_OK;
    CHECK(app_interface_step(app, 30000, &in, &out) == PICTORUS_OK);
    CHECK(out.button == 0.0 && out.vbat_a == 1);
    CHECK(app_interface_peripheral_status(app, &status) == PICTORUS_OK);
    CHECK(status.pwr_btn.error_count == 1);

    app_interface_deinit(app);
    free(mem);
}

static void test_unset_functions_count_as_failures(void) {
    AppPeripherals p = {0};
    void *mem = alloc_instance();
    AppInterface *app = NULL;
    CHECK(app_interface_init(mem, &p, &app) == PICTORUS_OK);

    AppInputs in = {.setpoint = 1.0};
    AppOutputs out = {0};
    CHECK(app_interface_step(app, 10000, &in, &out) == PICTORUS_ERR_PERIPHERAL);

    AppPeripheralStatus status;
    CHECK(app_interface_peripheral_status(app, &status) == PICTORUS_OK);
    CHECK(status.pwr_btn.error_count == 1);
    CHECK(status.status_led.error_count == 1);
    CHECK(status.motor_pwm.error_count == 1);
    CHECK(status.vbat.error_count == 1);
    CHECK(status.analog_out.error_count == 1);

    app_interface_deinit(app);
    free(mem);
}

int main(void) {
    test_runtime();
    test_init_validation();
    test_peripherals_and_independent_instances();
    test_failures_are_reported_but_do_not_change_the_model();
    test_unset_functions_count_as_failures();
    printf("ok\n");
    return 0;
}
