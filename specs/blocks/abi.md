# C ABI

Status: Rev 5, 2026-10-13. ABI version 1, experimental: no binary
compatibility promise until P3 closes.
Rev 1: §2, §3, §4.3, §8 (P1 Phase Review).
Rev 2: §1.10, §8 (P1 Phase Review round 2).
Rev 3: §1.10 names the dynamic library (round 3).
Rev 4: §3 run messages (round 8).
Rev 5: §1, §3, §4, §6, §7, §8 host capabilities, string values, link
tests (P3; D20 to D22). §6.2 grants without script declarations, §6.3
host declarations (D20 revised).

`include/ulua.h` declares every item of this file. The value type
carries every family from version 1, so a later phase adds a family
without a second call path.

## 1. Conventions

1. Every function returns `ulua_status` (`int32_t`), except
   `ulua_abi_version`.
2. Objects are opaque pointers: `ulua_context`, `ulua_module`,
   `ulua_instance`, `ulua_capability`. A module, an instance, and a
   capability belong to the context that made them. An object of
   another context as an argument gives `ULUA_INVALID_ARGUMENT`.
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
7. A call into a context from inside one of its host callbacks (the
   output callback or a host function of §6.2) gives `ULUA_BUSY`. The
   free functions and `ulua_capability_revoke` also give `ULUA_BUSY`
   there and do nothing.
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
code `host:output`, `host:capability`, `host:revoked`, or `host:result`
(§6). The message of a `host:capability`, `host:revoked`, or
`host:result` diagnostic is the name of the capability. `ULUA_INTERNAL` has the code `internal`. If the OS
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
3. An export has `NUMBER`, `BOOLEAN`, and `STRING` parameters, and a
   `VOID`, `NUMBER`, `BOOLEAN`, or `STRING` result. A module with an
   export of another type gives `ULUA_REJECTED` with the code
   `unsupported`. The check reads the export signature that the module
   carries.
4. A `STRING` argument of `ulua_call` that is not valid UTF-8 gives
   `ULUA_INVALID_ARGUMENT` before any script runs. The call copies the
   bytes into the instance and charges them to `max_memory_bytes`.
5. A `STRING` result of `ulua_call` points to bytes that the instance
   owns. They stay valid until the next call that takes the instance or
   its context, other than the module queries of §7. The bytes are
   valid UTF-8 (collisions.md C5).
6. A `NUMBER` value is any binary64, NaN and -0 included. A `BOOLEAN`
   value is 0 or 1.

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

### 6.1 Output

`print(text)` calls `output` once with the bytes of `text`, without a
line terminator. A non-zero return stops the run with `ULUA_HOST_ERROR`.
The bytes are valid until `output` returns.

### 6.2 Capabilities (D20, D21, D22)

```c
typedef int32_t (*ulua_host_fn)(void* userdata, const ulua_value* arguments,
    size_t argument_count, ulua_value* result);

typedef struct ulua_capability_desc {
    uint32_t struct_size;
    uint32_t reserved;
    const char* name;           /* ASCII identifier */
    size_t name_length;
    const uint32_t* parameter_types;
    size_t parameter_count;
    uint32_t result_type;
    uint32_t reserved2;
    ulua_host_fn function;
    void* userdata;
} ulua_capability_desc;
```

1. A capability is one host function with a name and a signature.
   `ulua_capability_new` copies the descriptor. The name is 1 to 64
   bytes and matches `[A-Za-z_$][A-Za-z0-9_$]*`. It is not a reserved
   word of ES2022 and not a prelude name (`print`, `Utf8`, and the
   global names of the ES2022 library that collisions.md accepts).
   There are at most `ULUA_MAX_HOST_PARAMETERS` (16) parameters. Each
   parameter type is `NUMBER`, `BOOLEAN`, or `STRING`. The result type
   is `VOID`, `NUMBER`, `BOOLEAN`, or `STRING`. `function` is not null.
   Else the call gives `ULUA_INVALID_ARGUMENT`.
2. **Grant.** `ulua_compile` takes a list of capabilities: the grants.
   Two grants with one name, or a revoked grant, give
   `ULUA_INVALID_ARGUMENT`. Each grant is a host function of the
   script, with the name and the signature of the capability
   (collisions.md C18). A grant that the script does not call is valid
   and unused. The module holds a reference to each grant that the
   script calls; each instance of the module holds the same
   references.
3. **Call.** A script call of a host function calls `function` once,
   on the thread of the run, with the arguments in source order. The
   argument values follow §4. A `STRING` argument points to bytes that
   stay valid until `function` returns. `result` is zeroed, with the
   type `VOID`.
4. **Result.** `function` returns 0 and writes `result` with the
   result type of the capability and §4.1. A `STRING` result points to
   valid UTF-8 bytes. They stay valid until the next host callback on
   the context, or until the next call of a function of this file on
   the context, whichever is first. The runtime copies them before the
   script continues, and charges them to `max_memory_bytes`. A result
   that breaks this rule stops the run with `ULUA_HOST_ERROR` and the
   code `host:result`.
5. **Error.** A non-zero return stops the run with `ULUA_HOST_ERROR`
   and the code `host:capability` (D22). No script code runs after
   it: no `catch` and no `finally` (collisions.md C17.7). The instance
   keeps the state that the run reached (§7.8).
6. **Revocation.** After `ulua_capability_revoke` returns, no call
   reaches `function`. A script call of a revoked capability stops the
   run with `ULUA_HOST_ERROR` and the code `host:revoked`, before any
   argument reaches the host. Revocation is permanent. A revoked
   capability stays valid as a pointer until `ulua_capability_free`.
7. **Free.** `ulua_capability_free` revokes the capability and
   releases the host reference. The pointer is then invalid. A module
   or instance that holds the capability keeps its own reference and
   sees it as revoked.
8. **Costs.** Each host function call charges one step. The time and
   the memory inside `function` are outside the run limits: the host
   is trusted (`CLAUDE.md` invariant 2).
9. `function` does not unwind: a C++ exception or a Rust panic that
   leaves it is undefined behaviour (a host precondition). A panic in
   the Rust code of this library around the call gives `ULUA_INTERNAL`
   (§1.8).

### 6.3 Host declarations (D20)

`ulua_declarations` writes the TypeScript declarations of a grant list.
The host gives the text, as a `.d.ts` file, to script authors and to
`tsc` (`CLAUDE.md` invariant 3).

1. The text is UTF-8 with LF line ends. Line 1 is
   `// Ulua host declarations. Generated by ulua_declarations.` Then
   one line per grant, in grant order:
   `declare function <name>(p0: <T>, p1: <T>, ...): <R>;`
   `<T>` is `number`, `boolean`, or `string`; `<R>` is also `void`.
   The text ends with a line end. The same grant list gives the same
   bytes.
2. Two grants with one name give `ULUA_INVALID_ARGUMENT`. A revoked
   grant is written: a declaration is a type, not an authority.
3. A capacity below the text length gives `ULUA_INVALID_ARGUMENT`,
   writes only `*length`, and writes no byte. A null buffer is valid
   with capacity 0, to read the length.
4. A script that `tsc` checks with this text and the prelude, and that
   `ulua_compile` compiles with the same grants, sees one signature per
   host function on both sides.

## 7. Functions

```c
uint32_t ulua_abi_version(void);  /* 1 */
ulua_status ulua_default_compile_limits(ulua_compile_limits* out);
ulua_status ulua_default_run_limits(ulua_run_limits* out);

ulua_status ulua_context_new(ulua_context** out);
ulua_status ulua_context_free(ulua_context* context);

ulua_status ulua_capability_new(ulua_context* context, const ulua_capability_desc* desc,
    ulua_capability** out);
ulua_status ulua_capability_revoke(ulua_capability* capability);
ulua_status ulua_capability_free(ulua_capability* capability);
ulua_status ulua_declarations(ulua_capability* const* grants, size_t grant_count,
    char* buffer, size_t capacity, size_t* length);

ulua_status ulua_compile(ulua_context* context, const char* source, size_t length,
    ulua_capability* const* grants, size_t grant_count,
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

1. `ulua_context_free` frees each module, instance, and capability of
   the context.
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

Link tests (P3) run in the default build:

1. A C11 program and a C++17 program include `include/ulua.h` and
   compile with `-Wall -Wextra -Wpedantic -Werror`. Each calls every
   function of §7, with one capability of each result type, and checks
   the results.
2. Each program links once against the static library and once against
   the dynamic library, and runs.
3. The exported symbols of the dynamic library equal the functions of
   §7 (§1.10). A symbol check reads them from the built library.
