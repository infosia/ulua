# C ABI

Status: Rev 4, 2026-10-12. ABI version 1, experimental: no binary
compatibility promise until P3 closes.
Rev 1: §2, §3, §4.3, §8 (P1 Phase Review).
Rev 2: §1.10, §8 (P1 Phase Review round 2).
Rev 3: §1.10 names the dynamic library (round 3).
Rev 4: §3 run messages (round 8).

`include/ulua.h` declares every item of this file. The value type
carries every family from version 1, so a later phase adds a family
without a second call path.

## 1. Conventions

1. Every function returns `ulua_status` (`int32_t`), except
   `ulua_abi_version`.
2. Objects are opaque pointers: `ulua_context`, `ulua_module`,
   `ulua_instance`. A module and an instance belong to the context that
   made them.
3. Every descriptor struct starts with `uint32_t struct_size`. The
   caller sets it to `sizeof` of the struct. A different value gives
   `ULUA_INVALID_ARGUMENT`.
4. A null pointer where the function needs a pointer gives
   `ULUA_INVALID_ARGUMENT`, and the function writes no output. A null
   limits pointer selects the defaults.
5. Strings cross as a pointer and a byte length, UTF-8, without a
   terminator. A null pointer is valid only with length 0.
6. The host serializes all calls on one context and its objects. Calls
   on different contexts can run on different threads at the same time.
7. A call into a context from inside its output callback gives
   `ULUA_BUSY`.
8. No Rust panic, C++ exception, or Luau error crosses the ABI. A
   caught panic gives `ULUA_INTERNAL`. After `ULUA_INTERNAL` the
   context is poisoned: each later call on it, except the free
   functions, gives `ULUA_POISONED`.
9. Each free function accepts null and does nothing.
10. The dynamic library exports only the functions of §7. The static
    library also holds the Luau objects and the Rust runtime with their
    own symbols; a host that links Luau itself uses the dynamic
    library. Each native symbol of this project other than §7 has the
    prefix `ulua_internal_`. Test-only native code is not in either
    library.

## 2. Status codes

| Name | Value | Meaning |
|---|---|---|
| `ULUA_OK` | 0 | Success |
| `ULUA_INVALID_ARGUMENT` | 1 | A precondition of this file is false |
| `ULUA_REJECTED` | 2 | The compiler rejects the source; the diagnostic has the code |
| `ULUA_FAILED` | 3 | A run stops at a trap or a run limit, or the OS refuses a resource; the diagnostic has the code |
| `ULUA_HOST_ERROR` | 4 | The output callback returned a non-zero status |
| `ULUA_BUSY` | 5 | A call during a callback of the same context |
| `ULUA_INTERNAL` | 6 | A defect of this library |
| `ULUA_POISONED` | 7 | The context had `ULUA_INTERNAL` before |

## 3. Diagnostic

```c
typedef struct ulua_diagnostic {
    uint32_t struct_size;
    uint32_t reserved;          /* 0 */
    char code[32];              /* NUL-terminated ASCII, pipeline.md §3.7 */
    char message[256];          /* NUL-terminated UTF-8, cut at a scalar boundary */
    uint64_t offset;            /* source byte offset, or UINT64_MAX */
    uint32_t line;              /* 1-based, or 0 */
    uint32_t column;            /* 1-based, in bytes, or 0 */
} ulua_diagnostic;
```

A function with a `ulua_diagnostic*` parameter writes it on
`ULUA_REJECTED`, `ULUA_FAILED`, `ULUA_HOST_ERROR`, and `ULUA_INTERNAL`.
The parameter can be null. Run codes: `trap:<kind>` (corpus.md §6),
`limit:memory`, `limit:steps`, `limit:stack`. `ULUA_HOST_ERROR` has the
code `host:output`. `ULUA_INTERNAL` has the code `internal`. If the OS
refuses the compiler thread, `ulua_compile` gives `ULUA_FAILED` with the
code `host:resources` and does not poison the context. In P1 a run diagnostic
has offset `UINT64_MAX`. A run diagnostic message holds no chunk name and
no line of the generated Luau (`CLAUDE.md` invariant 6).

## 4. Values

```c
enum { ULUA_TYPE_VOID = 0, ULUA_TYPE_NUMBER = 1, ULUA_TYPE_BOOLEAN = 2,
       ULUA_TYPE_STRING = 3 };

typedef struct ulua_value {
    uint32_t type;
    uint32_t boolean;     /* BOOLEAN: 0 or 1 */
    double number;        /* NUMBER: any binary64 */
    const char* data;     /* STRING: UTF-8 bytes */
    size_t length;        /* STRING: byte count */
} ulua_value;
```

1. Each field that the type does not use is zero or null. Else the call
   gives `ULUA_INVALID_ARGUMENT`.
2. An argument type must equal the parameter type of the export.
3. P1 accepts `NUMBER` and `BOOLEAN` parameters, and `VOID`, `NUMBER`,
   and `BOOLEAN` results. A module with an export of another type gives
   `ULUA_REJECTED` with the code `unsupported`. The check reads the
   export signature that the module carries. A direct test compiles a
   `string` parameter and a `string` result.

## 5. Limits

```c
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
    uint64_t max_memory_bytes;  /* per instance */
    uint64_t max_steps;         /* per ulua_instance_new or ulua_call */
} ulua_run_limits;
```

`pipeline.md` §6 states each limit.

## 6. Host

```c
typedef int32_t (*ulua_output_fn)(void* userdata, const char* text, size_t length);

typedef struct ulua_host {
    uint32_t struct_size;
    uint32_t reserved;
    void* userdata;
    ulua_output_fn output;      /* null: print discards the text */
} ulua_host;
```

`print(text)` calls `output` once with the bytes of `text`, without a
line terminator. A non-zero return stops the run with `ULUA_HOST_ERROR`.
The bytes are valid until `output` returns.

## 7. Functions

```c
uint32_t ulua_abi_version(void);  /* 1 */
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
ulua_status ulua_module_export_signature(const ulua_module* module, uint32_t index,
    uint32_t* parameter_types, size_t capacity, size_t* parameter_count, uint32_t* result_type);
ulua_status ulua_module_find_export(const ulua_module* module, const char* name,
    size_t length, uint32_t* index);

ulua_status ulua_instance_new(ulua_context* context, const ulua_module* module,
    const ulua_host* host, const ulua_run_limits* limits, ulua_instance** out,
    ulua_diagnostic* diagnostic);
ulua_status ulua_instance_free(ulua_instance* instance);
ulua_status ulua_call(ulua_instance* instance, uint32_t export_index,
    const ulua_value* arguments, size_t argument_count, ulua_value* result,
    ulua_diagnostic* diagnostic);
```

1. `ulua_context_free` frees each module and instance of the context.
   After it, each pointer to them is invalid.
2. `ulua_module_free` with live instances of the module: the instances
   stay valid. An instance does not read its module after
   `ulua_instance_new`.
3. Export order is source order. An export name is valid until the
   module is freed.
4. `ulua_module_export_signature` with a capacity below the parameter
   count gives `ULUA_INVALID_ARGUMENT` and writes only
   `parameter_count` and `result_type`.
5. `ulua_module_find_export` with an unknown name gives
   `ULUA_INVALID_ARGUMENT` and writes `UINT32_MAX`.
6. `ulua_instance_new` runs the module-level statements. On failure no
   instance exists and `*out` is null.
7. `ulua_call` with a wrong argument count or type gives
   `ULUA_INVALID_ARGUMENT` before any script runs.
8. After `ULUA_FAILED` or `ULUA_HOST_ERROR`, the instance keeps the
   state that the run reached. A later call on it is valid.
9. The host thread of `ulua_instance_new` and `ulua_call` has a stack
   of at least `ULUA_MIN_RUN_STACK` bytes (`pipeline.md` §7.4). The
   header defines the value.

## 8. Tests

Each function has a direct unit test (CLAUDE.md principle 1). A test
compiles a C file against `include/ulua.h` and checks `sizeof` and
each field offset of each struct against the Rust definition. It also
checks the value of each status code, each type tag,
`ULUA_MIN_RUN_STACK`, and the native value of `ALLOCATION_STEP_BYTES`
against the Rust constants. The test runs in the default build.
