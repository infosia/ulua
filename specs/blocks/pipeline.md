# Pipeline

Status: Rev 0, 2026-10-11. Decisions D7, D9 to D13 (`plan.md` §8).

This file states the compiler and runtime structure. `abi.md` states
the C ABI. `collisions.md` states the lowering of each form.

## 1. Workspace

| Path | Content |
|---|---|
| `Cargo.toml` | The workspace. Every `oxc_*` crate comes from one pin: https://github.com/infosia/oxc, `rev = "43ad6ed5043f524f78c16a2d874b5fc588942a15"`. |
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
   `COMPILER_STACK_BYTES`. P1 measures the value (§7).
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

## 6. Limits

| Limit | Applies to | Diagnostic or failure |
|---|---|---|
| `max_source_bytes` | `ulua_compile` | `limit:source-bytes` |
| `max_nesting_depth` | `ulua_compile`: parser guard and every compiler recursion | `limit:nesting-depth` |
| C15 bounds | `ulua_compile` | `limit:<name>` (collisions.md C15) |
| `max_memory_bytes` | each instance: the budget of its `lua_Alloc` | `limit:memory` |
| `max_steps` | each `ulua_instance_new` and `ulua_call`: interrupt callbacks | `limit:steps` |
| Luau call depth | each call (`LUAI_MAXCALLS`, `LUAI_MAXCCALLS`) | `limit:stack` |

1. A zero limit is a zero limit. It is never "no limit".
2. The library has a ceiling for `max_source_bytes` and
   `max_nesting_depth`. A request above the ceiling uses the ceiling.
3. `max_steps` counts interrupt callbacks, not wall time.
4. Compiler memory is outside `max_memory_bytes`. The source ceiling
   bounds it; P1 measures it.

## 7. Measurements of P1

P1 measures each value below, records it in
`specs/tracking/p1-skeleton.md`, and pins it with a test at and past
the value.

1. `COMPILER_STACK_BYTES`, and the ceiling of `max_nesting_depth`: the
   deepest form at the ceiling compiles in a Debug build with half of
   the stack unused.
2. The ceiling of `max_source_bytes`.
3. The C15 bounds and their limit names.
4. The stack that `ulua_call` needs on the host thread: Luau at
   `LUAI_MAXCCALLS` in a Debug build.
5. Compiler memory at the source ceiling.

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
  compiled entry to `<dir>/<name>.luau`.
- It runs `ulua_instance_new`, then the export `main` with no
  arguments. It uses the default limits.
