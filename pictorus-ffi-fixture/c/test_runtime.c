/* Exercises the runtime, peripherals and app lifecycle the way an integrator would. */
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

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
    uint8_t read_data[2];
    pictorus_status_t result;
    int write_reads, writes;
    uint8_t read_addr, read_wr[4];
    size_t read_wr_len, read_len;
    uint8_t write_addr, written[8];
    size_t written_len;
} fake_i2c_t;

typedef struct {
    uint8_t read_data[3];
    pictorus_status_t result;
    int reads, writes;
    uint8_t written[8];
    size_t written_len;
} fake_spi_t;

/* Models a receive ring: reads hand over what has arrived, then it is empty. */
typedef struct {
    uint8_t incoming[16];
    size_t incoming_len;
    pictorus_status_t result;
    int reads, writes;
    size_t offered;
    uint8_t written[8];
    size_t written_len;
} fake_serial_t;

typedef struct {
    pictorus_can_frame_t queue[4];
    size_t queued, next;
    pictorus_status_t result;
    int receives, transmits;
    pictorus_can_frame_t sent;
} fake_can_t;

typedef struct {
    uint8_t datagrams[4][16];
    size_t lens[4];
    size_t queued, next;
    pictorus_status_t result;
    int recvs, sends;
    char destination[32];
    uint8_t written[8];
    size_t written_len;
} fake_udp_t;

typedef struct {
    fake_button_t button;
    fake_led_t led;
    fake_pwm_t pwm;
    fake_adc_t adc;
    fake_dac_t dac;
    fake_i2c_t i2c;
    fake_spi_t spi;
    fake_serial_t serial;
    fake_can_t can;
    fake_udp_t udp;
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

static pictorus_status_t i2c_write_read(void *ctx, uint8_t addr, const uint8_t *wr,
                                        size_t wr_len, uint8_t *rd, size_t rd_len) {
    fake_i2c_t *f = ctx;
    f->write_reads++;
    f->read_addr = addr;
    f->read_wr_len = wr_len;
    memcpy(f->read_wr, wr, wr_len < 4 ? wr_len : 4);
    f->read_len = rd_len;
    if (f->result == PICTORUS_OK) memcpy(rd, f->read_data, rd_len < 2 ? rd_len : 2);
    return f->result;
}

static pictorus_status_t i2c_write(void *ctx, uint8_t addr, const uint8_t *wr,
                                   size_t wr_len) {
    fake_i2c_t *f = ctx;
    f->writes++;
    f->write_addr = addr;
    f->written_len = wr_len;
    memcpy(f->written, wr, wr_len < 8 ? wr_len : 8);
    return f->result;
}

static pictorus_status_t spi_read(void *ctx, uint8_t *rx, size_t len) {
    fake_spi_t *f = ctx;
    f->reads++;
    if (f->result == PICTORUS_OK) memcpy(rx, f->read_data, len < 3 ? len : 3);
    return f->result;
}

static pictorus_status_t spi_write(void *ctx, const uint8_t *tx, size_t len) {
    fake_spi_t *f = ctx;
    f->writes++;
    f->written_len = len;
    memcpy(f->written, tx, len < 8 ? len : 8);
    return f->result;
}

static pictorus_status_t serial_read(void *ctx, uint8_t *buf, size_t cap, size_t *len) {
    fake_serial_t *f = ctx;
    f->reads++;
    f->offered = cap;
    if (f->result != PICTORUS_OK) return f->result;
    if (f->incoming_len == 0) return PICTORUS_EMPTY;
    size_t n = f->incoming_len < cap ? f->incoming_len : cap;
    memcpy(buf, f->incoming, n);
    f->incoming_len = 0;
    *len = n;
    return PICTORUS_OK;
}

static pictorus_status_t serial_write(void *ctx, const uint8_t *buf, size_t len) {
    fake_serial_t *f = ctx;
    f->writes++;
    f->written_len = len;
    memcpy(f->written, buf, len < 8 ? len : 8);
    return f->result;
}

static pictorus_status_t can_receive(void *ctx, pictorus_can_frame_t *frame) {
    fake_can_t *f = ctx;
    f->receives++;
    if (f->result != PICTORUS_OK) return f->result;
    if (f->next >= f->queued) return PICTORUS_EMPTY;
    *frame = f->queue[f->next++];
    return PICTORUS_OK;
}

static pictorus_status_t can_transmit(void *ctx, const pictorus_can_frame_t *frame) {
    fake_can_t *f = ctx;
    f->transmits++;
    f->sent = *frame;
    return f->result;
}

static pictorus_status_t udp_recv(void *ctx, uint8_t *buf, size_t cap, size_t *len) {
    fake_udp_t *f = ctx;
    f->recvs++;
    if (f->result != PICTORUS_OK) return f->result;
    if (f->next >= f->queued) return PICTORUS_EMPTY;
    size_t n = f->lens[f->next] < cap ? f->lens[f->next] : cap;
    memcpy(buf, f->datagrams[f->next], n);
    f->next++;
    *len = n;
    return PICTORUS_OK;
}

static pictorus_status_t udp_send(void *ctx, const char *destination, const uint8_t *buf,
                                  size_t len) {
    fake_udp_t *f = ctx;
    f->sends++;
    strncpy(f->destination, destination, sizeof f->destination - 1);
    f->written_len = len;
    memcpy(f->written, buf, len < 8 ? len : 8);
    return f->result;
}

static AppPeripherals peripherals_for(fake_board_t *board) {
    AppPeripherals p = {
        .pwr_btn = {.ctx = &board->button, .read = button_read},
        .status_led = {.ctx = &board->led, .write = led_write},
        .motor_pwm = {.ctx = &board->pwm, .set = pwm_set},
        .vbat = {.ctx = &board->adc, .read = adc_read},
        .analog_out = {.ctx = &board->dac, .write = dac_write},
        .imu_bus = {.ctx = &board->i2c,
                    .write_read = i2c_write_read,
                    .write = i2c_write},
        .flash = {.ctx = &board->spi, .read = spi_read, .write = spi_write},
        .console = {.ctx = &board->serial, .read = serial_read, .write = serial_write},
        .can0 = {.ctx = &board->can, .receive = can_receive, .transmit = can_transmit},
        .telemetry = {.ctx = &board->udp, .recv = udp_recv, .send = udp_send},
    };
    return p;
}

static void queue_can(fake_can_t *f, uint32_t id, bool extended, const uint8_t *data,
                      uint8_t dlc) {
    pictorus_can_frame_t *frame = &f->queue[f->queued++];
    memset(frame, 0, sizeof *frame);
    frame->id = id;
    frame->extended = extended;
    frame->dlc = dlc;
    memcpy(frame->data, data, dlc);
}

static void queue_udp(fake_udp_t *f, const char *data) {
    f->lens[f->queued] = strlen(data);
    memcpy(f->datagrams[f->queued], data, strlen(data));
    f->queued++;
}

static void *alloc_instance(void) {
    void *mem = aligned_alloc(app_interface_align(), app_interface_size());
    CHECK(mem != NULL);
    return mem;
}

static bool near(double a, double b) { return fabs(a - b) < 1e-9; }

static void init_one(fake_board_t *board, void **mem, AppInterface **app) {
    AppPeripherals p = peripherals_for(board);
    *mem = alloc_instance();
    CHECK(app_interface_init(*mem, &p, app) == PICTORUS_OK);
}

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

/* The header and Rust must lay the shared types out the same way. */
static void test_layout_agrees_with_rust(void) {
    CHECK(sizeof(pictorus_can_frame_t) == fixture_can_frame_size());
    CHECK(sizeof(pictorus_peripheral_status_t) == fixture_peripheral_status_size());
    CHECK(sizeof(AppPeripherals) == fixture_peripherals_size());
    CHECK(sizeof(AppOutputs) == fixture_outputs_size());
}

static void test_peripheral_config(void) {
    CHECK(APP_PERIPHERAL_CONFIG.imu_bus.frequency_hz == 400000);
    CHECK(APP_PERIPHERAL_CONFIG.flash.mode == 3);
    CHECK(APP_PERIPHERAL_CONFIG.flash.bit_order == PICTORUS_SPI_MSB_FIRST);
    CHECK(APP_PERIPHERAL_CONFIG.console.baud_rate == 115200);
    CHECK(APP_PERIPHERAL_CONFIG.can0.bitrate == 500000);
    CHECK(strcmp(APP_PERIPHERAL_CONFIG.telemetry.address, "0.0.0.0:9001") == 0);
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

static void test_simple_peripherals_and_independent_instances(void) {
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

static void test_byte_peripherals(void) {
    static const uint8_t can_a[] = {5};
    static const uint8_t can_b[] = {7, 8};
    fake_board_t board = {
        .i2c = {.read_data = {0xA1, 0xA2}},
        .spi = {.read_data = {0xB1, 0xB2, 0xB3}},
        .serial = {.incoming = "abc", .incoming_len = 3},
    };
    queue_can(&board.can, 0x100, false, can_a, 1);
    queue_can(&board.can, 0x123, false, can_b, 2);
    queue_udp(&board.udp, "one");
    queue_udp(&board.udp, "two");

    void *mem;
    AppInterface *app;
    init_one(&board, &mem, &app);

    AppInputs in = {.setpoint = 0.0};
    AppOutputs out = {0};
    CHECK(app_interface_step(app, 10000, &in, &out) == PICTORUS_OK);

    /* I2C: the command byte is written, then two bytes are read, in one transaction. */
    CHECK(board.i2c.write_reads == 1);
    CHECK(board.i2c.read_addr == 104 && board.i2c.read_wr_len == 1);
    CHECK(board.i2c.read_wr[0] == 117 && board.i2c.read_len == 2);
    CHECK(out.imu.len == 2 && out.imu.first == 0xA1);
    /* An output block writes its command byte followed by the data. */
    CHECK(board.i2c.writes == 1 && board.i2c.write_addr == 104);
    CHECK(board.i2c.written_len == 3);
    CHECK(board.i2c.written[0] == 107 && board.i2c.written[1] == 1 &&
          board.i2c.written[2] == 2);

    /* SPI: two blocks, one read. */
    CHECK(board.spi.reads == 1);
    CHECK(out.flash_a.len == 3 && out.flash_a.first == 0xB1);
    CHECK(out.flash_b.len == 3 && out.flash_b.first == 0xB1);
    CHECK(board.spi.writes == 1 && board.spi.written_len == 1 &&
          board.spi.written[0] == 0xAA);

    /* Serial: two blocks, one read, which was offered a whole buffer. */
    CHECK(board.serial.reads == 1 && board.serial.offered > 3);
    CHECK(out.console_a.len == 3 && out.console_a.first == 'a');
    CHECK(out.console_b.len == 3 && out.console_b.first == 'a');
    CHECK(board.serial.writes == 1 && board.serial.written_len == 2);
    CHECK(memcmp(board.serial.written, "hi", 2) == 0);

    /* CAN: the queue is drained, and the block gets the frame for its identifier. */
    CHECK(board.can.receives == 3);
    CHECK(out.can.len == 2 && out.can.first == 7);
    CHECK(board.can.transmits == 1);
    CHECK(board.can.sent.id == 0x321 && !board.can.sent.extended);
    CHECK(board.can.sent.dlc == 3 && board.can.sent.data[2] == 3);

    /* UDP: the most recent datagram wins, and the destination comes from the block. */
    CHECK(board.udp.recvs == 3);
    CHECK(out.telemetry.len == 3 && out.telemetry.first == 't');
    CHECK(board.udp.sends == 1);
    CHECK(strcmp(board.udp.destination, "10.0.0.1:9000") == 0);
    CHECK(board.udp.written_len == 1 && board.udp.written[0] == 9);

    /* The next tick asks again, finds everything drained, and sees no data. */
    CHECK(app_interface_step(app, 20000, &in, &out) == PICTORUS_OK);
    CHECK(board.spi.reads == 2 && board.serial.reads == 2);
    CHECK(board.can.receives == 4 && board.udp.recvs == 4);
    CHECK(out.console_a.len == 0 && out.can.len == 0 && out.telemetry.len == 0);
    /* The I2C bus is not cached, so each tick performs its own transaction. */
    CHECK(board.i2c.write_reads == 2);

    AppPeripheralStatus status;
    CHECK(app_interface_peripheral_status(app, &status) == PICTORUS_OK);
    CHECK(status.imu_bus.error_count == 0 && status.flash.error_count == 0);
    CHECK(status.console.error_count == 0 && status.can0.error_count == 0);
    CHECK(status.telemetry.error_count == 0);

    app_interface_deinit(app);
    free(mem);
}

static void test_failures_are_reported_but_do_not_change_the_model(void) {
    fake_board_t board = {.button = {.level = true}, .adc = {.sample = 800}};
    void *mem;
    AppInterface *app;
    init_one(&board, &mem, &app);

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

static void test_byte_peripheral_failures(void) {
    fake_board_t board = {
        .i2c = {.read_data = {1, 2}, .result = PICTORUS_ERR_PERIPHERAL},
        .spi = {.read_data = {1, 2, 3}, .result = PICTORUS_ERR_PERIPHERAL},
        .serial = {.incoming = "abc", .incoming_len = 3, .result = PICTORUS_ERR_PERIPHERAL},
        .can = {.result = PICTORUS_ERR_PERIPHERAL},
        .udp = {.result = PICTORUS_ERR_PERIPHERAL},
    };
    void *mem;
    AppInterface *app;
    init_one(&board, &mem, &app);

    AppInputs in = {.setpoint = 0.0};
    AppOutputs out = {0};
    CHECK(app_interface_step(app, 10000, &in, &out) == PICTORUS_ERR_PERIPHERAL);

    /* A failed read is no data, not stale or garbage bytes. */
    CHECK(out.imu.len == 0 && out.flash_a.len == 0 && out.flash_b.len == 0);
    CHECK(out.console_a.len == 0 && out.console_b.len == 0);
    CHECK(out.can.len == 0 && out.telemetry.len == 0);

    /* Reads that failed were not retried by the second block in the same tick. */
    CHECK(board.spi.reads == 1 && board.serial.reads == 1);
    CHECK(board.can.receives == 1 && board.udp.recvs == 1);

    /* Each peripheral failed once to read or receive, and once to write or transmit. */
    AppPeripheralStatus status;
    CHECK(app_interface_peripheral_status(app, &status) == PICTORUS_OK);
    CHECK(status.imu_bus.error_count == 2);
    CHECK(status.flash.error_count == 2);
    CHECK(status.console.error_count == 2);
    CHECK(status.can0.error_count == 2);
    CHECK(status.telemetry.error_count == 2);
    CHECK(status.imu_bus.last_error == PICTORUS_ERR_PERIPHERAL);

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
    CHECK(status.imu_bus.error_count == 2);
    CHECK(status.flash.error_count == 2);
    CHECK(status.console.error_count == 2);
    CHECK(status.can0.error_count == 2);
    CHECK(status.telemetry.error_count == 2);

    app_interface_deinit(app);
    free(mem);
}

int main(void) {
    test_runtime();
    test_layout_agrees_with_rust();
    test_peripheral_config();
    test_init_validation();
    test_simple_peripherals_and_independent_instances();
    test_byte_peripherals();
    test_failures_are_reported_but_do_not_change_the_model();
    test_byte_peripheral_failures();
    test_unset_functions_count_as_failures();
    printf("ok\n");
    return 0;
}
