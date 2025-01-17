/* Generated with cbindgen:0.24.3 */

/* Warning, this file is autogenerated by cbindgen. Don't modify this manually. */

#include <stdint.h>
#include <Python.h>

typedef enum ComponentState {
    PRE_INITIALIZED = 0,
    READY = 1,
    STARTING = 2,
    RUNNING = 3,
    STOPPING = 4,
    STOPPED = 5,
    RESUMING = 6,
    RESETTING = 7,
    DISPOSING = 8,
    DISPOSED = 9,
    DEGRADING = 10,
    DEGRADED = 11,
    FAULTING = 12,
    FAULTED = 13,
} ComponentState;

typedef enum ComponentTrigger {
    INITIALIZE = 1,
    START = 2,
    START_COMPLETED = 3,
    STOP = 4,
    STOP_COMPLETED = 5,
    RESUME = 6,
    RESUME_COMPLETED = 7,
    RESET = 8,
    RESET_COMPLETED = 9,
    DISPOSE = 10,
    DISPOSE_COMPLETED = 11,
    DEGRADE = 12,
    DEGRADE_COMPLETED = 13,
    FAULT = 14,
    FAULT_COMPLETED = 15,
} ComponentTrigger;

typedef enum LogColor {
    NORMAL = 0,
    GREEN = 1,
    BLUE = 2,
    MAGENTA = 3,
    CYAN = 4,
    YELLOW = 5,
    RED = 6,
} LogColor;

typedef enum LogLevel {
    DEBUG = 10,
    INFO = 20,
    WARNING = 30,
    ERROR = 40,
    CRITICAL = 50,
} LogLevel;

typedef struct LiveClock LiveClock;

typedef struct Logger_t Logger_t;

typedef struct Rc_String Rc_String;

typedef struct TestClock TestClock;

typedef struct TestClockAPI {
    struct TestClock *_0;
} TestClockAPI;

typedef struct LiveClockAPI {
    struct LiveClock *_0;
} LiveClockAPI;

/**
 * Logger is not C FFI safe, so we box and pass it as an opaque pointer.
 * This works because Logger fields don't need to be accessed, only functions
 * are called.
 */
typedef struct CLogger {
    struct Logger_t *_0;
} CLogger;

/**
 * Represents a time event occurring at the event timestamp.
 */
typedef struct TimeEvent_t {
    /**
     * The event name.
     */
    struct Rc_String *name;
    /**
     * The event ID.
     */
    UUID4_t event_id;
    /**
     * The message category
     */
    uint64_t ts_event;
    /**
     * The UNIX timestamp (nanoseconds) when the object was initialized.
     */
    uint64_t ts_init;
} TimeEvent_t;

/**
 * Represents a time event and its associated handler.
 */
typedef struct TimeEventHandler_t {
    /**
     * The event.
     */
    struct TimeEvent_t event;
    /**
     * The event ID.
     */
    PyObject *callback_ptr;
} TimeEventHandler_t;

struct TestClockAPI test_clock_new(void);

void test_clock_drop(struct TestClockAPI clock);

/**
 * # Safety
 * - Assumes `callback_ptr` is a valid PyCallable pointer.
 */
void test_clock_register_default_handler(struct TestClockAPI *clock, PyObject *callback_ptr);

void test_clock_set_time(struct TestClockAPI *clock, uint64_t to_time_ns);

double test_clock_timestamp(struct TestClockAPI *clock);

uint64_t test_clock_timestamp_ms(struct TestClockAPI *clock);

uint64_t test_clock_timestamp_us(struct TestClockAPI *clock);

uint64_t test_clock_timestamp_ns(struct TestClockAPI *clock);

PyObject *test_clock_timer_names(const struct TestClockAPI *clock);

uintptr_t test_clock_timer_count(struct TestClockAPI *clock);

/**
 * # Safety
 * - Assumes `name_ptr` is a valid C string pointer.
 * - Assumes `callback_ptr` is a valid PyCallable pointer.
 */
void test_clock_set_time_alert_ns(struct TestClockAPI *clock,
                                  const char *name_ptr,
                                  uint64_t alert_time_ns,
                                  PyObject *callback_ptr);

/**
 * # Safety
 * - Assumes `name_ptr` is a valid C string pointer.
 * - Assumes `callback_ptr` is a valid PyCallable pointer.
 */
void test_clock_set_timer_ns(struct TestClockAPI *clock,
                             const char *name_ptr,
                             uint64_t interval_ns,
                             uint64_t start_time_ns,
                             uint64_t stop_time_ns,
                             PyObject *callback_ptr);

/**
 * # Safety
 * - Assumes `set_time` is a correct `uint8_t` of either 0 or 1.
 */
CVec test_clock_advance_time(struct TestClockAPI *clock, uint64_t to_time_ns, uint8_t set_time);

void vec_time_event_handlers_drop(CVec v);

/**
 * # Safety
 * - Assumes `name_ptr` is a valid C string pointer.
 */
uint64_t test_clock_next_time_ns(struct TestClockAPI *clock, const char *name_ptr);

/**
 * # Safety
 * - Assumes `name_ptr` is a valid C string pointer.
 */
void test_clock_cancel_timer(struct TestClockAPI *clock, const char *name_ptr);

void test_clock_cancel_timers(struct TestClockAPI *clock);

struct LiveClockAPI live_clock_new(void);

void live_clock_drop(struct LiveClockAPI clock);

double live_clock_timestamp(struct LiveClockAPI *clock);

uint64_t live_clock_timestamp_ms(struct LiveClockAPI *clock);

uint64_t live_clock_timestamp_us(struct LiveClockAPI *clock);

uint64_t live_clock_timestamp_ns(struct LiveClockAPI *clock);

const char *component_state_to_cstr(enum ComponentState value);

/**
 * Returns an enum from a Python string.
 *
 * # Safety
 * - Assumes `ptr` is a valid C string pointer.
 */
enum ComponentState component_state_from_cstr(const char *ptr);

const char *component_trigger_to_cstr(enum ComponentTrigger value);

/**
 * Returns an enum from a Python string.
 *
 * # Safety
 * - Assumes `ptr` is a valid C string pointer.
 */
enum ComponentTrigger component_trigger_from_cstr(const char *ptr);

const char *log_level_to_cstr(enum LogLevel value);

/**
 * Returns an enum from a Python string.
 *
 * # Safety
 * - Assumes `ptr` is a valid C string pointer.
 */
enum LogLevel log_level_from_cstr(const char *ptr);

const char *log_color_to_cstr(enum LogColor value);

/**
 * Returns an enum from a Python string.
 *
 * # Safety
 * - Assumes `ptr` is a valid C string pointer.
 */
enum LogColor log_color_from_cstr(const char *ptr);

/**
 * Creates a new logger.
 *
 * # Safety
 * - Assumes `trader_id_ptr` is a valid C string pointer.
 * - Assumes `machine_id_ptr` is a valid C string pointer.
 * - Assumes `instance_id_ptr` is a valid C string pointer.
 */
struct CLogger logger_new(const char *trader_id_ptr,
                          const char *machine_id_ptr,
                          const char *instance_id_ptr,
                          enum LogLevel level_stdout,
                          enum LogLevel level_file,
                          uint8_t file_logging,
                          const char *directory_ptr,
                          const char *file_name_ptr,
                          const char *file_format_ptr,
                          const char *component_levels_ptr,
                          uint8_t is_bypassed);

void logger_drop(struct CLogger logger);

const char *logger_get_trader_id_cstr(const struct CLogger *logger);

const char *logger_get_machine_id_cstr(const struct CLogger *logger);

UUID4_t logger_get_instance_id(const struct CLogger *logger);

uint8_t logger_is_bypassed(const struct CLogger *logger);

/**
 * Log a message.
 *
 * # Safety
 * - Assumes `component_ptr` is a valid C string pointer.
 * - Assumes `msg_ptr` is a valid C string pointer.
 */
void logger_log(struct CLogger *logger,
                uint64_t timestamp_ns,
                enum LogLevel level,
                enum LogColor color,
                const char *component_ptr,
                const char *msg_ptr);

/**
 * # Safety
 * - Assumes `name` is borrowed from a valid Python UTF-8 `str`.
 */
struct TimeEvent_t time_event_new(const char *name,
                                  UUID4_t event_id,
                                  uint64_t ts_event,
                                  uint64_t ts_init);

struct TimeEvent_t time_event_clone(const struct TimeEvent_t *event);

void time_event_drop(struct TimeEvent_t event);

const char *time_event_name_to_cstr(const struct TimeEvent_t *event);

/**
 * Returns a [`TimeEvent`] as a C string pointer.
 */
const char *time_event_to_cstr(const struct TimeEvent_t *event);

struct TimeEventHandler_t dummy(struct TimeEventHandler_t v);
