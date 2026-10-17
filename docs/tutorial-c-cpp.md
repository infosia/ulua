# Ulua for C and C++ developers

Ulua is a scripting language for embedding in a native application.
Its syntax is a subset of TypeScript. A Rust compiler checks a script
and emits typed Luau source; the pinned Luau VM runs it inside your
process. You drive it through one C header, `include/ulua.h`, and
every function of that header returns a status.

Ulua treats each script as hostile. Your host sets the limits, grants
each function that a script can call, and decides when script code
runs. No thread and no event loop of Ulua runs script code behind
your back.

This tutorial assumes C, not TypeScript. Every command and every
output below comes from a run against the repository as committed.
The host of those runs is `aarch64-apple-darwin` with Apple clang.
The programs are in [`examples/`](../examples/), and a gate test
builds and runs each one on every gate run.

## Setup

From the repository root:

```sh
npm install                                   # once: the pinned TypeScript, for tsc
export LUAU_SOURCE_DIR=/path/to/luau          # checkout at luau-revision.txt
cargo build --release -p ulua
```

The build gives the static library `target/release/libulua.a` and a
dynamic library beside it. The dynamic library exports only the
`ulua_` functions of the header. Link a host against the static
library (macOS):

```sh
mkdir -p target/examples
cc -std=c11 -Wall -Wextra -Wpedantic -Werror examples/hello/host.c -I include \
    target/release/libulua.a -o target/examples/hello \
    -lc++ -liconv -framework Security -framework CoreFoundation
```

On Linux the tail of the link line is `-lstdc++ -lpthread -ldl -lm`.
The comment at the top of each `host.c` holds both lines.

## The objects

| Object | Made by | Holds |
|---|---|---|
| `ulua_context` | `ulua_context_new` | Everything below; one thread at a time |
| `ulua_capability` | `ulua_capability_new` | One host function that you can grant |
| `ulua_module` | `ulua_compile` | One compiled script and its export signatures |
| `ulua_instance` | `ulua_instance_new` | One running copy of a module, with its own heap and limits |

Free each one with its `_free` function, or free the context, which
frees all of them. Each free function accepts `NULL`.

## Step 1: the script

[`examples/hello/hello.ts`](../examples/hello/hello.ts) exports two
functions:

```ts
export function main(): void {
  print("hello from Ulua");
  const words: string[] = ["alpha", "beta", "gamma"];
  print(words.map((w: string): string => `<${w}>`).join(" "));
  const c = new Counter();
  c.add(2);
  print(`count ${c.add(3)}`);
  print(`length squared ${lengthSquared({ x: 3, y: 4 })}`);
}

export function greet(name: string): string {
  return `hello, ${name}`;
}
```

A module exports zero or more functions. `main` has no special
meaning; your host calls each export by name.

## Step 2: compile, instantiate, call

The core of [`examples/hello/host.c`](../examples/hello/host.c):

```c
static int32_t output(void* userdata, const char* text, size_t length) {
    (void)userdata;
    return fwrite(text, 1, length, stdout) == length && putchar('\n') != EOF ? 0 : 1;
}

    ulua_diagnostic diagnostic = {.struct_size = sizeof(diagnostic)};
    ulua_host host = {.struct_size = sizeof(host), .output = output};
    ulua_value result = {0};
    ulua_value argument = {.type = ULUA_TYPE_STRING, .data = "world", .length = 5};

    CHECK(ulua_context_new(&context));
    CHECK(ulua_compile(context, source, source_length, NULL, 0, NULL, &module, &diagnostic));
    CHECK(ulua_instance_new(context, module, &host, NULL, &instance, &diagnostic));
    CHECK(ulua_module_find_export(module, "main", 4, &main_index));
    CHECK(ulua_module_find_export(module, "greet", 5, &greet_index));
    CHECK(ulua_call(instance, main_index, NULL, 0, &result, &diagnostic));
    CHECK(ulua_call(instance, greet_index, &argument, 1, &result, &diagnostic));
```

- Every descriptor struct starts with `struct_size`; set it to
  `sizeof` of the struct.
- `ulua_compile` takes the source as bytes and a length, a list of
  grants (none here), and limits (`NULL` selects the defaults). A
  rejection gives `ULUA_REJECTED` and fills the diagnostic: a code
  such as `C4`, a message, and a line and a column.
- `ulua_instance_new` runs the module-level statements. `print`
  calls your `output` function once per line, without the line end.
- `ulua_value` carries `VOID`, `NUMBER`, `BOOLEAN`, or `STRING`. Each
  field that the type does not use is zero. A string result points to
  bytes that the instance owns; use them before your next call on the
  context.

```sh
$ target/examples/hello examples/hello/hello.ts
compile hello
create instance with output callback
call main
hello from Ulua
<alpha> <beta> <gamma>
count 5
length squared 25
call greet(world)
greet result: hello, world
free resources
```

## Step 3: grant host functions

A script reaches your host only through the functions that you grant
to `ulua_compile`.
[`examples/capabilities/game.ts`](../examples/capabilities/game.ts)
calls `damage` and `log`:

```ts
let health: number = 100;

export function hit(power: number): number {
  const amount = damage(power);
  health -= amount;
  log(`hit for ${amount}, health ${health}`);
  return health;
}
```

Each grant is a capability: a name, a signature, and a C function.
From [`examples/capabilities/host.c`](../examples/capabilities/host.c):

```c
static int32_t damage(void* userdata, const ulua_value* arguments,
                      size_t count, ulua_value* result) {
    (void)userdata;
    if (count != 1 || arguments[0].type != ULUA_TYPE_NUMBER) return 1;
    result->type = ULUA_TYPE_NUMBER;
    result->number = arguments[0].number * 2;
    return 0;
}

    ulua_capability_desc damage_desc = {
        .struct_size = sizeof(damage_desc), .name = "damage", .name_length = 6,
        .parameter_types = &number_type, .parameter_count = 1,
        .result_type = ULUA_TYPE_NUMBER, .function = damage
    };

    CHECK(ulua_capability_new(context, &damage_desc, &grants[0]));
    CHECK(ulua_capability_new(context, &log_desc, &grants[1]));
    CHECK(ulua_declarations(grants, 2, declarations, sizeof(declarations), &declaration_length));
    CHECK(ulua_compile(context, source, source_length, grants, 2, NULL, &module, &diagnostic));
```

`ulua_declarations` writes the TypeScript declarations of a grant
list. Save them as a `.d.ts` file beside the scripts, so `tsc` and
the editor of the script author know the host functions. A script
that calls a function you did not grant is rejected: `tsc` reports
TS2304, and `ulua_compile` reports `C18`.

A non-zero return from a host function stops the run with
`ULUA_HOST_ERROR` and the code `host:capability`. A script cannot
catch it. `ulua_capability_revoke` takes effect at once: the next call
from any instance stops with `host:revoked`, and your function is not
called.

```sh
$ target/examples/capabilities examples/capabilities/game.ts
create damage and log capabilities
write declarations
// Ulua host declarations. Generated by ulua_declarations.
declare function damage(p0: number): number;
declare function log(p0: string): void;
declarations match
compile game with grants
create instance
call hit(10)
host log: hit for 20, health 80
hit result: 80
call hit(10)
host log: hit for 20, health 60
hit result: 60
call heal
host log: healed
heal result: true
revoke damage
call hit(10)
hit status: ULUA_HOST_ERROR, code: host:revoked
free resources
```

## Step 4: limits, and what survives a stop

A run stops at a trap, at a limit, or at a host error. The instance
keeps the state that the run reached, and your next call on it is
valid. From [`examples/limits/host.c`](../examples/limits/host.c):

```c
    CHECK(ulua_default_run_limits(&limits));
    limits.max_steps = 10000;
    limits.max_memory_bytes = 1024 * 1024;
    CHECK(ulua_instance_new(context, module, &host, &limits, &instance, &diagnostic));
```

```sh
$ target/examples/limits examples/limits/limits.ts
compile limits
create instance: max_steps=10000 max_memory_bytes=1048576
spin: status=3 (ULUA_FAILED), code=limit:steps
grow: status=3 (ULUA_FAILED), code=limit:memory
outOfRange: status=3 (ULUA_FAILED), code=trap:index-out-of-range
ok: status=0 (ULUA_OK), result=3
free resources
```

| Limit | Set in | Bounds | Code |
|---|---|---|---|
| `max_source_bytes` | `ulua_compile_limits` | The source of one compilation | `limit:source-bytes` |
| `max_nesting_depth` | `ulua_compile_limits` | Nesting in the parser and every compiler pass | `limit:nesting-depth` |
| `max_steps` | `ulua_run_limits` | Each run that the host starts | `limit:steps` |
| `max_memory_bytes` | `ulua_run_limits` | The heap of one instance | `limit:memory` |
| `max_open_requests` | `ulua_run_limits` | Open async requests of one instance | `limit:requests` |

The compiler adds fixed bounds of its own, each with a `limit:` code
(for example `limit:upvalues` past 192 upvalues in one function), so
the generated Luau stays inside the Luau limits. A trap has the code
`trap:<kind>`; an exception that the script does not catch has the
code `trap:uncaught-exception` and the message `name: message`. No
stop reaches a `catch` or a `finally` of the script.

## Step 5: async host functions

A host function can return a promise. Give the capability a `start`
function in place of `function`. A script call gives the promise and
calls `start` with a request id; you complete the request later.
[`examples/async/scores.ts`](../examples/async/scores.ts):

```ts
async function total(ids: number[]): Promise<number> {
  const scores = await Promise.all(ids.map((id: number): Promise<number> => fetchScore(id)));
  return scores.reduce((sum: number, s: number): number => sum + s, 0);
}

export async function report(): Promise<number> {
  print("requesting scores");
  try {
    const sum = await total([1, 2, 3]);
    print(`total ${sum}`);
    return sum;
  } finally {
    print("report done");
  }
}
```

From [`examples/async/host.c`](../examples/async/host.c):

```c
static int32_t fetch_score(void* userdata, const ulua_value* arguments,
                           size_t count, uint64_t request) {
    request_queue* queue = userdata;
    if (count != 1 || arguments[0].type != ULUA_TYPE_NUMBER || queue->count == 3) return 1;
    queue->requests[queue->count++] = (pending_request){request, arguments[0].number};
    printf("start fetchScore(%.0f): request %" PRIu64 "\n", arguments[0].number, request);
    return ferror(stdout) ? 1 : 0;
}

    CHECK(ulua_call_start(instance, report_index, NULL, 0, &task, &diagnostic));
    status = ulua_task_result(instance, task, &result, &diagnostic);   /* ULUA_PENDING */
    for (size_t i = 0; i < queue.count; ++i) {
        pending_request pending = queue.requests[i];
        ulua_value score = {.type = ULUA_TYPE_NUMBER, .number = pending.id * 10};
        CHECK(ulua_request_complete(instance, pending.request, &score));
    }
    CHECK(ulua_pump(instance, 0, &settled, &diagnostic));
    CHECK(ulua_task_result(instance, task, &result, &diagnostic));     /* ULUA_OK */
```

```sh
$ target/examples/async examples/async/scores.ts
create async fetchScore capability
compile scores with grant
create instance with output callback
requests before call: open=0 recorded=0
call_start report
requesting scores
start fetchScore(1): request 1
start fetchScore(2): request 2
start fetchScore(3): request 3
task: 1
task_result: ULUA_PENDING
requests after call: open=3 recorded=0
complete request 1: score=10
complete request 2: score=20
complete request 3: score=30
requests before pump: open=3 recorded=3
pump
total 60
report done
pump settled: 3
requests after pump: open=0 recorded=0
task_result: ULUA_OK, total=60
free resources
```

- `ulua_request_complete` and `ulua_request_fail` record a result and
  run no script code. They need no memory from the script's heap, so
  a script that fills its heap cannot block them.
- `ulua_pump` settles the recorded results in the order you recorded
  them. Each settle is one run with its own `max_steps`, and the
  script's continuations run inside it.
- A failed request (`ulua_request_fail`) stops its settle with
  `host:request`; the script cannot catch it.
- `ulua_call_start` and `ulua_task_result` give the result of an
  async export when it settles. `ulua_task_discard` releases a task
  that a stop left pending.

## The rules a host must know

- **One thread per context at a time.** Serialize every call on a
  context and its objects. Two contexts can run on two threads at the
  same time.
- **No calls back from a callback.** A call into a context from inside
  its output function or a host function gives `ULUA_BUSY`, and does
  nothing.
- **No unwinding through Ulua.** No Rust panic, no C++ exception, and
  no Luau error crosses the ABI. Your host functions must not unwind
  either.
- **`ULUA_INTERNAL` poisons the context.** It is a defect of this
  library. Each later call on that context gives `ULUA_POISONED`; free
  it and make a new one.
- **A stack for runs.** The thread of `ulua_instance_new`, `ulua_call`,
  and `ulua_pump` has at least `ULUA_MIN_RUN_STACK` bytes of stack.
- **Strings are bytes and a length.** A string that crosses the ABI
  is UTF-8, with no terminator. Ulua checks the UTF-8 of every string
  that you pass in.
- **Lifetimes.** A module can be freed while its instances run. A
  capability that you free counts as revoked in every module that
  holds it. A string result is valid until your next call on the
  context.
- **`Math.random` is deterministic.** Each instance starts its
  sequence from a fixed default seed, so two runs of a script give the
  same numbers. To change it, set `random_seed_set = 1` and
  `random_seed` in `ulua_host`, or call `ulua_instance_seed_random`.
  Ulua reads no clock and no system entropy.
- **No compatibility promise.** The ABI is version 1 and can change
  with the header; rebuild your host against the header you link.

The full contract is [`specs/blocks/abi.md`](../specs/blocks/abi.md).

## Reading on

- [Ulua for TypeScript developers](tutorial-typescript.md): the
  language from the script author's side.
- [`examples/`](../examples/): the complete programs of this page,
  and the runner that the TypeScript tutorial uses.
- [`specs/blocks/abi.md`](../specs/blocks/abi.md): every function,
  status, and code.
