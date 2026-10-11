#ifndef ULUA_H
#define ULUA_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* ABI version 1 is experimental until P3 closes. */
typedef int32_t ulua_status;
enum { ULUA_OK=0, ULUA_INVALID_ARGUMENT=1, ULUA_REJECTED=2, ULUA_FAILED=3,
       ULUA_HOST_ERROR=4, ULUA_BUSY=5, ULUA_INTERNAL=6, ULUA_POISONED=7 };
enum { ULUA_TYPE_VOID=0, ULUA_TYPE_NUMBER=1, ULUA_TYPE_BOOLEAN=2, ULUA_TYPE_STRING=3 };
/* provisional: S3 measures */
#define ULUA_MIN_RUN_STACK (4u * 1024u * 1024u)
typedef struct ulua_context ulua_context;
typedef struct ulua_module ulua_module;
typedef struct ulua_instance ulua_instance;
typedef struct ulua_diagnostic {
    uint32_t struct_size;
    uint32_t reserved;
    char code[32];
    char message[256];
    uint64_t offset;
    uint32_t line;
    uint32_t column;
} ulua_diagnostic;
typedef struct ulua_value {
    uint32_t type;
    uint32_t boolean;
    double number;
    const char* data;
    size_t length;
} ulua_value;
typedef struct ulua_compile_limits {
    uint32_t struct_size;
    uint32_t reserved;
    uint64_t max_source_bytes;
    uint32_t max_nesting_depth;
    uint32_t reserved2;
} ulua_compile_limits;
typedef struct ulua_run_limits {
    uint32_t struct_size;
    uint32_t reserved;
    uint64_t max_memory_bytes;
    uint64_t max_steps;
} ulua_run_limits;
typedef int32_t (*ulua_output_fn)(void* userdata, const char* text, size_t length);
typedef struct ulua_host {
    uint32_t struct_size;
    uint32_t reserved;
    void* userdata;
    ulua_output_fn output;
} ulua_host;
/* Set every descriptor struct_size to sizeof(descriptor), and reserved fields to zero.
   All pointers must refer to valid, aligned storage for the stated lifetime and length.
   Strings have UTF-8 bytes and no terminator. NULL is valid only with length zero.
   Serialize each context and its objects. Different contexts are independent.
   Callback reentry returns ULUA_BUSY. ULUA_INTERNAL poisons the context.
   Free functions accept NULL. Context free invalidates all child pointers.
   Module free preserves live instances. Export names live until module free.
   A diagnostic is optional. Run failures preserve instance state.
   Run calls require at least ULUA_MIN_RUN_STACK bytes of host stack. */
uint32_t ulua_abi_version(void);
ulua_status ulua_default_compile_limits(ulua_compile_limits* out);
ulua_status ulua_default_run_limits(ulua_run_limits* out);
ulua_status ulua_context_new(ulua_context** out);
ulua_status ulua_context_free(ulua_context* context);
ulua_status ulua_compile(ulua_context* context, const char* source, size_t length,
    const ulua_compile_limits* limits, ulua_module** out, ulua_diagnostic* diagnostic);
ulua_status ulua_module_free(ulua_module* module);
ulua_status ulua_module_export_count(const ulua_module* module, uint32_t* out);
ulua_status ulua_module_export_name(const ulua_module* module, uint32_t index,
    const char** name, size_t* length);
/* Insufficient capacity writes only parameter_count and result_type. */
ulua_status ulua_module_export_signature(const ulua_module* module, uint32_t index,
    uint32_t* parameter_types, size_t capacity, size_t* parameter_count, uint32_t* result_type);
/* An unknown name writes UINT32_MAX. */
ulua_status ulua_module_find_export(const ulua_module* module, const char* name,
    size_t length, uint32_t* index);
/* A run failure sets *out to NULL. A null limits pointer selects defaults. */
ulua_status ulua_instance_new(ulua_context* context, const ulua_module* module,
    const ulua_host* host, const ulua_run_limits* limits, ulua_instance** out, ulua_diagnostic* diagnostic);
ulua_status ulua_instance_free(ulua_instance* instance);
ulua_status ulua_call(ulua_instance* instance, uint32_t export_index,
    const ulua_value* arguments, size_t argument_count, ulua_value* result, ulua_diagnostic* diagnostic);
#ifdef __cplusplus
}
#endif
#endif
