#include "ulua.h"
/* Independent C compiler measurements of every version 1 layout field. */
size_t ulua_layout_probe(size_t index) {
    static const size_t layout[] = {
        sizeof(ulua_diagnostic),
        offsetof(ulua_diagnostic, struct_size),
        offsetof(ulua_diagnostic, reserved),
        offsetof(ulua_diagnostic, code),
        offsetof(ulua_diagnostic, message),
        offsetof(ulua_diagnostic, offset),
        offsetof(ulua_diagnostic, line),
        offsetof(ulua_diagnostic, column),
        sizeof(ulua_value),
        offsetof(ulua_value, type),
        offsetof(ulua_value, boolean),
        offsetof(ulua_value, number),
        offsetof(ulua_value, data),
        offsetof(ulua_value, length),
        sizeof(ulua_compile_limits),
        offsetof(ulua_compile_limits, struct_size),
        offsetof(ulua_compile_limits, reserved),
        offsetof(ulua_compile_limits, max_source_bytes),
        offsetof(ulua_compile_limits, max_nesting_depth),
        offsetof(ulua_compile_limits, reserved2),
        sizeof(ulua_run_limits),
        offsetof(ulua_run_limits, struct_size),
        offsetof(ulua_run_limits, reserved),
        offsetof(ulua_run_limits, max_memory_bytes),
        offsetof(ulua_run_limits, max_steps),
        sizeof(ulua_host),
        offsetof(ulua_host, struct_size),
        offsetof(ulua_host, reserved),
        offsetof(ulua_host, userdata),
        offsetof(ulua_host, output),
        ULUA_MIN_RUN_STACK
    };
    return index < sizeof(layout) / sizeof(layout[0]) ? layout[index] : SIZE_MAX;
}
