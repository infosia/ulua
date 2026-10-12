# Pipeline

Status: Rev 1, 2026-10-11. Decisions D7, D9 to D13 (`plan.md` §8).
Rev 1: the P1 values of §6 and §7 (`tracking/p1-skeleton.md`).
Rev 2: §3.1, §3.4, §3.10, §5.10 to §5.12, §6.3 and §6.5 (P1 Phase Review).
Rev 3: §3.10, §5.8, §5.10, §5.11, §6.3 (P1 Phase Review round 2).
Rev 4: the S5 forms of §5.8 and §5.10; `OUTPUT_FACTOR`; §7 heap value.
Rev 5: §3.10, §5.8, §5.10, §5.13 (P1 Phase Review round 3).
Rev 6: §4.6a, §7 values (round 8).
Rev 7: §5.14 functions as values, `limit:upvalues` (P2.1).
Rev 8: §5.14.3 parentheses for an optional function type (P2.1 review).
Rev 9: §5.14.3 nested aliases (P2.1 review 2).
Rev 10: §3.10 interned types and product shapes (P2.1 review 3).

This file states the compiler and runtime structure. `abi.md` states
the C ABI. `collisions.md` states the lowering of each form.

## 1. Workspace

| Path | Content |
|---|---|
| `Cargo.toml` | The workspace. Every `oxc_*` crate that the fork contains comes from one pin: https://github.com/infosia/oxc, `rev = "43ad6ed5043f524f78c16a2d874b5fc588942a15"`. A dependency of the fork from crates.io (for example `oxc_index`) keeps the version that the fork states. |
| `rust-toolchain.toml` | Toolchain `1.95.0`, components `rustfmt` and `clippy`. |
| `crates/ulua_compiler` | Parse, check, typed IR, and Luau emission. Safe Rust only (`#![forbid(unsafe_code)]`). No native code. |
| `crates/ulua` | The runtime and the C ABI. Crate types `rlib`, `staticlib`, `cdylib`. `build.rs` compiles Luau and the shim. |
| `crates/ulua_corpus` | The binary `ulua-corpus` for the gate (§8). It calls only the C ABI functions of `crates/ulua`. |
| `native/shim/` | The C++ shim (§4). |
| `include/ulua.h` | The C header (`abi.md`). |

## 2. Build

1. Cargo is the build root (D11). `crates/ulua/build.rs` compiles the
   Luau sources in `Common`, `Ast`, `Bytecode`, `Compiler`, and `VM`,
   and `native/shim/*.cpp`, with the `cc` crate.
2. `build.rs` reads the Luau source directory from the environment
   variable `LUAU_SOURCE_DIR`. If the variable is absent, the build
   fails with a message that names the variable.
3. `build.rs` compares `git -C $LUAU_SOURCE_DIR rev-parse HEAD` with
   `luau-revision.txt`. If they differ, the build fails and names both
   revisions.
4. Flags: `-std=c++17 -O3 -DNDEBUG -DLUA_USE_LONGJMP=1`; the VM unit
   adds `-fno-math-errno`. No other `LUA*` define.
5. No path of a developer machine enters a build output that the
   repository tracks.

## 3. Compiler

```text
source bytes -> UTF-8 check -> oxc_parser (guard) -> depth check
  -> checker -> typed IR -> Luau source -> luau_compile -> bytecode
```

1. **Thread.** `ulua_compile` runs the stages up to `luau_compile` on
   a thread that the library starts, with a fixed stack of
   `COMPILER_STACK_BYTES`. P1 measures the value (§7). If the OS
   refuses the thread, `ulua_compile` gives `ULUA_FAILED` with the
   code `host:resources` (`abi.md` §3). The context stays usable.
2. **Source.** The source is UTF-8. A byte sequence that is not UTF-8
   gives the diagnostic `syntax` at its first invalid byte. A source
   longer than `max_source_bytes` gives `limit:source-bytes` before
   any parse.
3. **Parser.** `oxc_parser` parses a TypeScript module with
   `ParseOptions::max_nesting_depth = Some(max_nesting_depth)`. A parse
   error gives `syntax`. `depth_exceeded` gives
   `limit:nesting-depth`.
4. **Depth.** The compiler never calls a recursive walk of `oxc_ast`,
   `oxc_ast_visit`, or `oxc_semantic`. Each recursive function of the
   compiler, over the AST, the IR, or the emitted text, takes one
   depth counter. It increments the counter before it recurses. At
   `max_nesting_depth` it returns `limit:nesting-depth`. A flat chain
   (`a+a+a`, `a.b.c`, `f()()`) is a nested AST and counts one level
   per link (D13).
   The checker counts the depth that the emitter and the emitted Luau
   reach. An input that the checker accepts never reaches a depth limit
   of the emitter. If it does, the result is `internal`. Each limit
   diagnostic carries the source offset of the form that reaches the
   limit.
5. **Checker.** The checker implements `surface.md` and
   `collisions.md`. It derives each type itself (C1). It never reads a
   type from `tsc`.
6. **One diagnostic.** The compiler stops at the first violation. A
   compile result carries one diagnostic: a code, a byte offset, a
   line, a column, and a message.
7. **Codes.** A diagnostic code is one of:
   - a rule id of `collisions.md` (`C1` to `C16`);
   - `syntax`;
   - `limit:source-bytes`, `limit:nesting-depth`, or a C15 bound
     `limit:<name>`;
   - `unsupported`: a form that `surface.md` lists and that the
     current phase does not implement.
8. **Luau compile.** `luau_compile` runs on the compiler thread, inside
   `catch (...)` (D12). A Luau compile error of generated source is a
   defect. It gives `ULUA_INTERNAL` with the Luau message.
9. **No panic.** A Rust panic on the compiler thread is caught at the
   thread join. It gives `ULUA_INTERNAL`.
10. **Linear time.** The time of each stage, `luau_compile` included,
    is at most O(n log n) in the source size n. A gate test compiles
    one input of each shape at n and at 4n bytes. The time at 4n is at
    most 8 times the time at n.
    The compiler stores each distinct type once and refers to it by an
    id. A type read, a type copy, a type equality test, and a type hash
    cost O(1). The growth shapes include the product of two sizes: a
    deep type read by many references, and every later pair that a
    slice adds.
    The generated Luau has at most `OUTPUT_FACTOR` bytes per source
    byte, plus a fixed prelude. The emitter checks the bound on each
    module. A module over the bound gives `internal`. §7 records the
    value and its margin over the largest measured ratio. The factor does not depend on the
    nesting depth or on the number of bindings. The shapes of the growth
    test include nested branches with many bindings and many writes.
    §7 records the compiler heap peak of the worst shape at the source
    ceiling.

## 4. Shim and the Luau C API

1. Luau builds with `LUA_USE_LONGJMP=1` (D12).
2. Rust calls only shim functions. Rust never calls a `lua_*`,
   `luaL_*`, or `luau_*` function directly.
3. Each shim entry is `extern "C"` and `noexcept`. Each `lua_*` call
   that can raise runs inside `lua_cpcall` or `lua_pcall`.
4. No shim frame between a raise and its catch holds an object with a
   destructor.
5. A host callback (the output capability) is a C++ trampoline. It
   calls the Rust function and receives a status. It raises the Luau
   error after the Rust frame returns.
6. After a memory-limit error, the shim runs a full collection before
   it returns.
6a. Each error that the pinned Luau VM raises for a size limit gives
   `limit:memory`, and each error for a depth limit gives `limit:stack`,
   whatever the host limits are. The shim holds the list of these
   errors, read from the pinned VM source, with the source location of
   each. Examples: "not enough memory", "string length overflow"
   (`MAXSSIZE`), "memory allocation error: block too big", "stack
   overflow". A test reaches each listed error that a P1 script can
   reach, through the ABI. Any other Luau error of a run is `internal`.
7. Each instance is one `lua_State` from `lua_newstate` with the
   budget allocator, `luaL_openlibs`, and `luaL_sandbox`. Each call
   runs on a thread from `luaL_sandboxthread`.

## 5. Emission

1. The output starts with `--!strict`. It type-checks under
   `luau-analyze` with exit status 0 (D10).
2. Every emitted identifier is generated (C14.5). Library functions are
   read once, at the top of the chunk, into generated locals.
3. A number in a template substitution or `String(n)` calls one
   generated function per chunk with the lowering of C4.5 (D9).
4. The chunk returns a table. Each key is an export name. Each value is
   the exported function.
5. Module-level statements run once, in `ulua_instance_new`.
6. The emitted chunk name is `=script`. A Luau error message carries no
   host path.
7. **Local budget (C15.1).** For each generated Luau function, the
   emitter counts every local that the function declares in all of its
   scopes: parameters, bindings, temporaries (C12), loop copies (C9.2),
   and generated locals. The count includes nested blocks. The budget
   is `LOCAL_BUDGET` = 176, below the Luau limit of 200 (§7).
8. **Spill table.** If the count exceeds `LOCAL_BUDGET`, the function
   declares one generated table local at its entry. Each binding past
   the budget, in source order, is a distinct generated field of that
   table. The table has a table type with one typed field per spilled
   binding, so `luau-analyze` checks each access. Each read and write
   of the binding is a field access of the table. C10 makes each write
   come before each read.
   The spill form stays inside §5.1 at the source ceiling: a function
   and a module chunk with the most bindings that fit the ceiling pass
   `luau-analyze`. The spilled bindings go to tables of at most 128
   fields each. Each table local has a table type with one typed field
   per binding. Its initial value is a table constructor with the
   typed default of each field, as an unspilled local has. If the fixed locals
   leave too few locals for the tables, the tables take more fields.
   The local count of §5.7 includes each table.
   Parameters, temporaries, and generated locals never spill. A
   function within the budget declares no table.
9. **Upvalues.** A function reads a module-level binding as an
   upvalue, or through the spill table of the module chunk. Its
   upvalue count is at most `LOCAL_BUDGET` plus the generated locals of
   the chunk. §5.14 extends this rule for nested functions and arrows.

10. **Typed writes.** Each assignment to a generated local or to a
    spill field has the form `name = (expr) :: T`. `T` is the declared
    Luau type of the target. Without the assertion, `luau-analyze`
    fails on chains of writes inside branches (`tracking/p1-skeleton.md`).
    A join write `name = (name) :: T` follows a branch or a loop for a
    binding that the statement writes and that is live after it. The
    emitter defers the join writes. It emits each pending join write
    once, before the next read of the binding or before the next
    branch. The number of join writes obeys the output bound of §3.10.
    The pending set costs O(1) amortized per write and per statement.
    No statement copies or merges the whole pending set.
    A function type in a write assertion is a generated type alias.
11. **Analyzer stress gate.** The gate emits one module per stress
    shape. Each module repeats its shape up to the source ceiling.
    `luau-analyze` checks each one with exit status 0. A shape is in
    the set only if the analyzer fails on it when the emitter drops a
    form of §5.8 or §5.10; the tracking note records that failure.
    Shapes: writes inside branches (`&&`, `||`, `?:`, `if`, `while`),
    nested branches with many bindings, and the most bindings in a
    function and in a module chunk.
12. **Template work.** A template of at most 16 parts lowers to one
    `..` expression. A template of more parts lowers to `table.concat`
    of one table constructor. Each byte of a part is copied at most
    twice.
13. **Number literals.** A number literal is emitted in the shortest
    decimal form that reads back as the same binary64 value.
14. **Functions as values (P2.1).**
    1. A captured binding is a binding that a nested function or arrow
       reads or writes. The emitter declares each captured binding as
       a Luau local at the entry of the block that declares it, with
       its typed default value. Each execution of the block gives a
       new local, so a closure in a loop body sees the binding of its
       own iteration (C9). A binding that no closure captures keeps the
       P1 form.
    2. A nested function declaration is a captured binding of its
       block in this sense. The emitter assigns each function
       declaration of a block at the block entry, after the locals of
       rule 1 and before the first statement, in source order. So a
       hoisted call (C10.3) finds its value, and the declarations of
       one block can call each other.
    3. An arrow lowers to a Luau function expression with typed
       parameters and a typed result. A function type lowers to a Luau
       function type through a generated type alias (§5.10). A function
       type alias names each nested function type by its own alias,
       so the alias text is linear in the source. A function
       type inside an optional type is in parentheses:
       `((number) -> boolean)?`. Without them, Luau reads
       `(number) -> boolean?` as a function with an optional result.
    4. The upvalues of a generated function are the distinct captured
       locals of enclosing functions that it reads or writes; a spill
       table counts once. A function with more upvalues than
       `UPVALUE_BUDGET` gives `limit:upvalues` (§6). P2.1 measures the
       budget against the Luau limit of 200.
    5. The local budget of §5.7 and the temporaries bound apply to each
       generated function on its own. A captured binding past the
       budget spills (§5.8); a closure reads it through the spill
       table.
    6. A call through a function value is a plain Luau call. C10 makes
       the value non-nil at each call. A function value never crosses
       the C ABI (`abi.md` §4).
    7. Luau allows 2^15 closures per function. A 65,536-byte source
       holds fewer function expressions, so no bound is needed; P2.1
       records the count.

## 6. Limits

| Limit | Applies to | Diagnostic or failure |
|---|---|---|
| `max_source_bytes` | `ulua_compile` | `limit:source-bytes` |
| `max_nesting_depth` | `ulua_compile`: parser guard and every compiler recursion | `limit:nesting-depth` |
| C15 bounds | `ulua_compile` | `limit:<name>` (collisions.md C15) |
| `max_memory_bytes` | each instance: the budget of its `lua_Alloc` | `limit:memory` |
| `max_steps` | each `ulua_instance_new` and `ulua_call`: interrupt callbacks | `limit:steps` |
| Luau call depth | each call (`LUAI_MAXCALLS`, `LUAI_MAXCCALLS`) | `limit:stack` |

C15 bounds of P1, each a fixed compiler constant:

| Code | Bound | Value |
|---|---|---|
| `limit:parameters` | parameters of one function | 32 |
| `limit:arguments` | arguments of one call | 32 |
| `limit:template-parts` | literal parts and substitutions of one template literal, empty literal parts included | 1025 |
| `limit:temporaries` | temporaries, default-parameter copies, and generated locals of one generated function; they never spill (§5.8) | 136 |
| `limit:upvalues` | upvalues of one generated function (§5.14) | 192 |

`limit:nesting-depth` also bounds the nesting of the generated Luau
below the Luau parser recursion limit of 1000. A bound that S3 finds
necessary and that this table does not name is a contract question.

1. A zero limit is a zero limit. It is never "no limit".
2. The library has a ceiling for `max_source_bytes` and
   `max_nesting_depth`. A request above the ceiling uses the ceiling.
3. `max_steps` counts interrupt callbacks. Each allocation also
   charges `ceil(bytes / ALLOCATION_STEP_BYTES)` steps. At the limit the
   allocator returns null and the run gives `limit:steps` with the
   message `Step limit exceeded`. A reallocation to an equal or smaller
   size is never charged and never fails: Luau requires this
   (`VM/src/lmem.cpp` of the pin). The charge
   bounds the time of one step by a constant, so `max_steps` bounds
   the run time. P1 measures `ALLOCATION_STEP_BYTES` (§7).
4. Compiler memory is outside `max_memory_bytes`. The source ceiling
   bounds it; P1 measures it.
5. `max_memory_bytes` counts allocated bytes, garbage included. A
   collection runs only when Luau starts it.

## 7. Measurements of P1

P1 measures each value below, records it in
`specs/tracking/p1-skeleton.md`, and pins it with a test at and past
the value.

1. `COMPILER_STACK_BYTES`, and the ceiling of `max_nesting_depth`: the
   deepest form at the ceiling compiles in a Debug build with half of
   the stack unused.
2. The ceiling of `max_source_bytes`.
3. The C15 bounds and their limit names, and `LOCAL_BUDGET`.
4. The stack that `ulua_call` needs on the host thread: Luau at
   `LUAI_MAXCCALLS` in a Debug build.
5. Compiler memory at the source ceiling.

P1 values (constants in `ulua_compiler` and `include/ulua.h`):

| Value | P1 | Measurement |
|---|---|---|
| `COMPILER_STACK_BYTES` | 2 MiB | Debug peak 716,864 B at the depth ceiling |
| Ceiling of `max_nesting_depth` | 48 | the deepest form passes with 1 MiB of stack in Debug |
| Ceiling of `max_source_bytes` | 65,536 | compiler heap peak 61.6 MB over 42 growth shapes and the round 9 shape (180 `let` bindings and a 41-link `%` chain, Release, P2.1) |
| `LOCAL_BUDGET` | 176 | Luau accepts 200 live locals |
| `ULUA_MIN_RUN_STACK` | 512 KiB | 130,544 B at `LUAI_MAXCCALLS` in Debug |
| Default `max_memory_bytes` | 64 MiB | minimum instance 401,816 B |
| Default `max_steps` | 1,000,000 | a test runs to the bound and one step past it |
| `OUTPUT_FACTOR` | 80, prelude 843 B | largest measured ratio 62.7 (`%` chain over a spilled binding, S7); margin 27% |
| `ALLOCATION_STEP_BYTES` | 256 | 1,000,000 charged steps of the slowest measured input take about 2.0 s in Release |

## 8. Corpus runner

`ulua-corpus <entry.ts>...` compiles and runs each entry through the C
ABI and prints one JSON line per entry:

```text
{"entry":"accept/a01-hello","status":"ok","output":["hello"],"code":null}
{"entry":"reject/r01-any","status":"rejected","output":[],"code":"C8"}
{"entry":"trap/t01-...","status":"failed","output":["..."],"code":"trap:index-out-of-range"}
```

- `status` is `ok`, `rejected`, `failed` (a trap or a limit of a run),
  or `internal`.
- With `--emit-luau <dir>`, it writes the generated Luau of each
  compiled entry to `<dir>/<name>.luau`. The generated Luau stays
  internal (invariant 6): the C ABI has no accessor for it. The runner
  calls the Rust function of `ulua_compiler` for this file only. The
  compile and the run use the C ABI.
- It runs `ulua_instance_new`, then the export `main` with no
  arguments. It uses the default limits.
