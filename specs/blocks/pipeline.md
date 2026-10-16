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
Rev 11: §4.8 traps, §5.15 arrays (P2.2).
Rev 12: §4.8 chunk argument, §3.10 densest shapes, §5.15 helpers (P2.2 review).
Rev 13: §3.10 `limit:output` (P2.2 review 2).
Rev 14: §5.16 array methods (P2.3).
Rev 15: §5.15.2 literal blocks, §6.0 limit positions, §3.10 final check (P2.2 review 3).
Rev 16: §5.7 loop locals end with their loop (P2.3).
Rev 17: §5.17 strings (P2.4).
Rev 18: §5.17.2 linear Luau-loop search, §6.3 library call rule (P2.4 review).
Rev 19: §5.18 nullable values and records (P2.5).
Rev 20: §6 `limit:fields` (P2.5 review 7).
Rev 21: §6 `limit:type-depth` (P2.5 review 8).
Rev 22: §6 recursive alias groups count their size (P2.5 review 9).
Rev 23: §6 a group counts every type link inside it (P2.5 review 10).
Rev 24: §6 `limit:fields` counts the methods of a class (P2.6).
Rev 25: §4.9 exceptions (P2.7).
Rev 26: §3.10 witness time of every growth shape, §4.9 protected closure
upvalues (P2.7 review 2).
Rev 27: §3.10 witness time at the source ceiling (P2.7 review 2).
Rev 28: §4.10 host functions, §8 grants (P3).
Rev 29: §4.10 one grant table (P3 review).
Rev 30: §4.11 async (P4.1).
Rev 31: §4.11 a stop clears the queue before the collection (P4.1 review).
Rev 32: §4.9 one completion helper (P4.1 review 3).
Rev 33: §6 `max_open_requests`, host-started runs (P4.3).
Rev 34: §2.6 sanitizer build (P5.1).
Rev 35: §9 fuzzing (P5.2).
Rev 36: §3.7 rule ids, §9.2 object pool, §9.3 slow-compile metric (P5.2).
Rev 37: §3.10 the timing gate (principle 17).
Rev 38: §9.3 witness and input deadlines, §9.4 reduction record (P5.2 review).
Rev 39: §10 fault injection (P5.3).
Rev 40: §6.4 compiler memory bound (P5.4).
Rev 41: §6.4 the test calls `ulua_compile`; a small-source base; native memory per shape (P5.4 review).
Rev 42: §5.19 interrupt points, §6.3 the instruction bound and GC steps (P5.5).
Rev 43: §5.19 loop exits, a derived instruction bound, the prelude, async shapes (P5.5 review 2).
Rev 44: §5.14.5a `CAPTURE_LOCALS`; §5.19 helper bodies, uncounted lines, the ratio bound 6 (P5.5 review 3).

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
6. **Sanitizer build (P5.1).** With `ULUA_SANITIZE=1`, `build.rs`
   compiles every native unit (Luau and the shim) with
   `-fsanitize=address,undefined -fno-omit-frame-pointer
   -fno-sanitize-recover=undefined -O1`, and links the sanitizer
   runtime into each test binary and example host. No other flag and
   no source changes. The sanitized build writes to its own target
   directory (`target/sanitize`), so it never mixes objects with the
   normal build. `sh tools/gate.sh sanitize` runs the tests, the Luau
   witnesses, the examples, and the docs witness on that build; any
   sanitizer report fails the gate.

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
   - a rule id of `collisions.md` (each `## Cn.` section: `C1` to `C19`);
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
    wide type read by many references, and every later pair that a
    slice adds. For each source construct, the growth test also
    measures its densest repetition (the shortest source form, nested
    or chained where the construct allows it) at the source ceiling,
    with generated names at their longest. The output check of the
    growth test runs at the ceiling.
    The generated Luau has at most `OUTPUT_FACTOR` bytes per source
    byte, plus a fixed prelude. The bound is a compiler limit
    (invariant 2): the emitter checks it as it writes, and the first
    statement that crosses it gives `limit:output` with the source
    offset of that statement. The final check also compares the
    counter with the length of the written text; a difference is
    `internal`. A measured ratio is a lower bound of the
    worst case only, so the bound is not a defect test. §7 records the
    value and the largest measured ratio. The factor does not depend on the
    nesting depth or on the number of bindings. The shapes of the growth
    test include nested branches with many bindings and many writes.
    §7 records the compiler heap peak of the worst shape at the source
    ceiling.
    The time of `luau-analyze -j1` over the output of each growth shape
    grows at most 12 times for 4 times the input from 4 KiB to 16 KiB.
    The timing gate (`sh tools/gate.sh timing`) asserts this ratio, and
    the compiler growth ratios of the growth tests. These ratios change
    only when the compiler's output changes, so the timing gate runs at
    the close of a slice and with each change to the checker or the
    emitter; `gate.sh full` does not run it (principle 17). At the
    ceiling of `max_source_bytes`,
    the time of each growth shape is at most 50 times the time of the
    reference shape `dense-assign`. Each Phase Review runs the
    `measurements` feature: it asserts the ceiling bound, prints each
    ratio from 16 KiB to 64 KiB, and the tracking note records them.
    The ceiling bounds the input, so a super-linear shape inside the
    ceiling bound is not a defect. A change of the ceiling measures
    every shape again. If no lowering form keeps a shape inside these
    bounds, a C15 bound with a measured value and a limit diagnostic
    takes its place. Measured at `57bd020`: N arrows
    `const g = (): number => { ... }` in one function take 0.72 s at
    16 KiB and 63.0 s at 64 KiB (ratio 88), with or without a `try`.
    Measured on one machine at the round 2 fix: the arrow growth shape takes
    124.3 s at 64 KiB before the fix and 0.45 s after it;
    `dense-assign` takes 0.32 s; the largest growth shape takes 9.73 s.

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
8. **Traps.** The generated code raises a trap through one native
   function of the shim. The shim passes the function as the first
   argument of the chunk; the chunk reads it from `...` into a
   generated local. The chunk calls no `getfenv` and reads no global
   for it: `getfenv` turns off the Luau safe-environment
   optimizations. The
   function takes the trap kind as a small integer, records it in the
   instance, and raises a Luau error. The shim maps the recorded kind to
   the code `trap:<kind>` (corpus.md §6). A script cannot call the
   function by name: every script name is generated (§5.2). The
   message holds no position of the generated Luau.
9. **Exceptions.** A `try` statement lowers to one protected closure
   per block (collisions.md C17). The closure captures the enclosing
   locals by reference and returns its completion as multiple values:
   a tag, a value, and a jump target where one is needed. Each
   protected call that returns `false` with a value that the private
   Error table does not hold raises that value again with
   `error(v, 0)` before any other code (C17.7); default `error(v)`
   adds a position and allocates. An Error value that leaves an
   exported function gives `ULUA_FAILED` with
   `trap:uncaught-exception` and the message `name: message`; it
   does not poison the context. Measured: `error(v, 0)` and
   `lua_error` request no allocation in the measured stops; nested
   protected calls reach "C stack overflow" at 198.
   One fixed prelude helper runs each protected call and its
   completion handling; the caller tests the completion tag only when
   the source block has a `return`, `break`, or `continue` that leaves
   it. Branch-local writes of completion locals at each call site made
   `luau-analyze` time grow with the loop depth (P4.1 review 3).
   A protected closure is a generated function for the upvalue budget
   (§5.14): its upvalues are the locals and upvalues of its owner that
   it reads or writes, and a `try` over the budget gives
   `limit:upvalues`, as a source function does. The witness time of
   `try` shapes follows §3.10. Measured at `3fafd70`:
   `try{a++;}finally{}` repeated in one function takes 0.13 s at 100,
   6.4 s at 400, and 65 s at 800 statements.
10. **Host functions.** The shim makes one native closure per grant
   that the module calls (collisions.md C18, `abi.md` §6.2). It
   passes them in one table, in grant order, as one chunk argument
   after the trap function: a chunk argument per grant reaches the
   Luau C-stack limit (`LUAI_MAXCSTACK`, 8,000) at 8,100 grants inside
   the source ceiling. The chunk reads each closure from the table
   into a generated local with its Luau function type, as for the
   trap function (§4.8): no global and no `getfenv`. The closure holds the
   capability reference of the instance. A call checks the revocation
   first, then converts the arguments, calls the C++ trampoline of
   §4.5, checks and copies the result, and raises a stop on an error.
   The stop is recorded in the instance before the raise, as for a
   trap, so a protected closure raises it again (C17.7).
   Each closure local is a module binding for the local budget (§5.8)
   and the upvalue budget (§5.14): a module that calls more grants
   than the budget spills them, and a function over the upvalue budget
   gives `limit:upvalues`. The witness time of host-call shapes follows
   §3.10.
11. **Async.** The prelude holds the promise table, the job queue, and
   the scheduler (collisions.md C19). Each `async` activation runs in a
   dedicated coroutine; the scheduler resumes it with
   `coroutine.resume`, never with `coroutine.wrap` or a direct yield
   from a host-started run (P4 Step 0 M1, M2). After each resume that
   returns `false`, the scheduler first reads the stop that the
   instance recorded and the native error class, and raises a stop
   again with `error(v, 0)` before any other code (C17.7); a value that
   the private Error table holds rejects the promise. The queue keeps a
   private head index and a tail index and clears each consumed entry,
   so a drain retains no completed job. The queue drains after each
   host-started run (`ulua_instance_new`, `ulua_call`,
   `ulua_call_start`, each settle of `ulua_pump`) inside the same
   protected call, under the same `max_steps` and `max_memory_bytes`.
   The unhandled-rejection check (C19.7) runs at the end of the drain.
   A stop during a run clears the job queue: no job of that run runs in
   a later run. The queue is cleared before the full collection of
   §4.6, so the collection can free the jobs and the promises that only
   the queue holds.
   Each `await` lowers to a yield through the private channel; the
   resumed value carries a type assertion to `T` at that one point
   (M3). The witness time of await shapes follows §3.10.

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
   P2.3 changes the count of loop copies and loop generated locals:
   the emitter declares them in a block around their loop, so they end
   with the loop, and the count is the largest number of such locals
   live at one point, not their sum. P2.3 first measures, with the
   pinned Luau, that a local in an ended block does not count toward
   the limit of 200 (the P1 gap of sequential loops). If the
   measurement contradicts this, the P1 count stays.
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
    5a. A generated function keeps at most `CAPTURE_LOCALS` captured
       bindings as Luau locals; each further captured binding of the
       function spills (§5.8). The Luau `NEWCLOSURE` of a capture by
       reference walks the open upvalues of the thread above the
       captured slot (`luaF_findupval`, `VM/src/lfunc.cpp`), so one
       closure creation over N captured locals of its own frame does
       work in proportion to N squared without allocation. Measured
       (P5.5 review 3, Release, a closure in `while (true)` over N
       captured `const` bindings, ratio to the `i++` baseline): N 10
       gives 5.0, 30 gives 18.1, 130 gives 120. `CAPTURE_LOCALS` (10) is
       the largest N whose ratio stays at most 5 (4.99 Debug, 4.94
       Release; 16 gives 7.35). After the rule, 130 and 190 captured
       bindings give 1.98.
    6. A call through a function value is a plain Luau call. C10 makes
       the value non-nil at each call. A function value never crosses
       the C ABI (`abi.md` §4).
    7. Luau allows 2^15 closures per function. A 65,536-byte source
       holds fewer function expressions, so no bound is needed; P2.1
       records the count.
15. **Arrays (P2.2).**
    1. `T[]` lowers to the Luau type `{T}`. A function element type is
       in parentheses where Luau needs them (§5.14.3).
    2. An array literal evaluates its elements in source order (C12).
       A constant element (a literal, `-` of a numeric literal) is
       written in place and takes no temporary. The other elements are
       evaluated in blocks of at most 32 temporaries; after each block
       the lowering appends the block to the array. So the values that
       a literal holds across its elements are at most 32 plus the
       array local, whatever its width; each element keeps the
       temporaries of its own expression. A literal of any width inside the source
       ceiling needs no width bound.
    The checks of rules 3 to 5 run in generated helper functions of the
    chunk prelude, one call per element access, so the emitted size of
    one access is a small constant.
    3. A read `xs[i]` evaluates `xs`, then `i` (C12). It traps
       `index-out-of-range` unless `i` is an integer in `[0, #xs)`; the
       test runs on `i` before `i + 1` (C6.1, C6.2). It then reads key
       `i + 1`.
    4. A write `xs[i] = e` evaluates `xs`, `i`, then `e`. Then it traps
       unless `i` is an integer in `[0, #xs]`, with `#xs` read after
       `e` (C6.3). It stores at key `i + 1`.
    5. A compound write `xs[i] op= e` reads the element with the rule 3
       check, evaluates `e`, and stores with the rule 4 check.
    6. `xs.length` lowers to `#xs`. No element is `nil` (C6.5) and no
       write makes a hole (rule 4), so the border is the length.
    7. `xs.push(e)` evaluates `xs`, then `e`, then reads `#xs`, then
       stores at key `#xs + 1` (C12.6).
    8. `for (const x of xs)` evaluates `xs` once into a temporary. It
       keeps a numeric counter from 1 and reads the live length before
       each step (C6.6). The binding `x` follows §5.14.1: a captured
       binding is a new local per step.
    9. An array value never crosses the C ABI (`abi.md` §4).
16. **Array methods (P2.3).**
    1. Each method of collisions.md C6.9 lowers to one call of a
       generated helper of the chunk prelude. A module holds the
       helpers that it uses and no other; the prelude of §3.10 is the
       base prelude plus the used helpers, a bounded constant.
    2. A helper takes its receiver and its arguments after the C12
       evaluation of the call: the receiver, then the arguments from
       left to right. It converts a number argument as ECMAScript
       ToIntegerOrInfinity converts it.
    3. A callback helper reads the length at its start, calls the
       callback with the value and, if the callback takes it, the
       zero-based index, and checks the length after each callback.
       A changed length raises the trap `array-modified` (§4.8).
    4. `sort` is a stable merge sort in the helper; it never calls
       `table.sort`, which is not stable and raises on an
       inconsistent comparator.
    5. `pop`, `shift`, and `at` raise `index-out-of-range` where C6.9
       says so. `xs.length = 0` clears the array in place.
    6. Each helper is generic in the element type, so `luau-analyze`
       checks each call. A helper loop counts toward `max_steps` as any
       loop does.
17. **Strings (P2.4).**
    1. Each string member of collisions.md C5 lowers to one call of a
       generated prelude helper, as array methods do (§5.16.1 and
       §5.16.2): only used helpers, C12 order, ToIntegerOrInfinity
       for number arguments.
    2. A helper works on byte offsets with `string.sub` and
       `string.byte`; it never uses a Luau pattern built from script
       text. A substring search (`indexOf`, `lastIndexOf`, `includes`,
       `split`, `replace`, `replaceAll`) is a linear-time algorithm
       written as a Luau loop (for example Knuth-Morris-Pratt, forward
       or backward), so its work counts toward `max_steps` (§6.3);
       `lastIndexOf` searches backward from its position. A boundary check tests that the byte at an
       offset is not a continuation byte (`0x80` to `0xBF`).
    3. `s.length` lowers to `#s`. `+` with a `number` operand calls the
       C4.5 format function; with a `boolean` operand it writes `true`
       or `false`.
    4. `for (const c of s)` steps by code point over the valid UTF-8 of
       `s`; a captured binding is a new local per step (§5.14.1).
    5. The traps `string-boundary`, `index-out-of-range`, and
       `invalid-argument` go through §4.8.
18. **Nullable values and records (P2.5).**
    1. `null` lowers to `nil`. `T | null` lowers to the Luau type `T?`,
       with a function type in parentheses (§5.14.3).
    2. An object type lowers to a Luau table type with one field per
       source field. A field name lowers to a generated key, one per
       distinct source field name in the module, so a script name never
       reaches the Luau source (C14.5). A `null` field is an absent
       key (C14.1).
    3. An object literal lowers to a table constructor; its field
       values are evaluated in source order (C12).
    4. A field read `o.f` and a field write `o.f = e` follow C12: `o`,
       then `e`, then the store.
    5. A null test is a comparison with `nil`. `a ?? b` lowers as C2
       states. `a?.f` and `a?.m(args)` evaluate `a` once.
    6. The checker computes null narrowing (C16) as a flow fact for
       each function: the true and false edges of a null test, the
       joins of `if`, loops, `&&`, `||`, and `?:`, the end of the
       narrowing at an assignment, and the C16 limits for fields and
       closures. A read of a `T | null` reference as `T` (a field read,
       a call, an argument of type `T`) needs the fact.
    7. Object values and nullable values never cross the C ABI
       (`abi.md` §4).

19. **Interrupt points (P5.5).** The pinned VM calls the interrupt
    only at a call, a return, and a loop back-edge
    (`VM/src/lvmexecute.cpp`). Straight-line code and forward jumps
    run without an interrupt. So the emitter bounds the straight-line
    work of each generated function.
    - The emitter counts the generated Luau statements of a function
      on each forward path since the last interrupt. A statement that
      always makes a Luau call ends the count.
    - At a join after branches, the count is the largest count of the
      branches. A loop body starts with the count before the loop. The
      count after the loop is the largest count of the loop entry, of
      the body end, and of each forward exit of the loop: each
      `break`, each `continue` lowered to an exit, and the exit of the
      loop condition. An interrupt point later in the body does not
      erase the count of an earlier exit.
    - Before the count passes `INTERRUPT_STATEMENTS` (16), the emitter
      emits `__ulua_interrupt()`, a call to an empty prelude function.
      It fires two interrupts (the call and the return). A repeated
      inline `for _ = 1, 1 do end` fires one, but raised the
      `luau-analyze` time of growth shapes (S4-module-bindings 1.47 s
      against 0.13 s at 16 KiB); `repeat until true` and `while false
      do end` are folded and fire none (P5.5, O1). The function is one
      module-level local, so it takes one of the module temporaries.
    - Each generated statement does a bounded number of instructions:
      the C15 bounds and the slot form of §5.8 bound it. A statement
      that the emitter cannot bound is a compiler defect. A pure
      expression emits at most 3 nodes per statement; a spill table
      header and the export table emit one field per statement. A
      long `&&`, `||`, or spill initializer was 525 to 4,117
      instructions in one statement before this rule. One node per
      statement added short-circuit branches with refinements, and
      `luau-analyze` failed with "Code is too complex to typecheck"
      on P25-logical-wrapped-tests at 16 KiB (P5.5).
    - A table store counts `STORE_WEIGHT` (8) statements, so two
      stores follow an interrupt point at most. A store does few
      instructions but costs more time per instruction: without the
      weight the largest ratio (below) was 10.59 (S4-function-
      bindings); weights 2, 4, 8 gave 7.98, 6.38, 4.77 (Release). The
      entry of a function with a 128-field spill header costs 488
      steps (32 before P5.5).
    - The bound is derived, not fitted. `STATEMENT_INSTRUCTIONS` is
      the largest instruction count of one generated statement; the
      C15 bounds give it (a call with 32 arguments: 33 moves and the
      call). `INTERRUPT_INSTRUCTIONS` is `INTERRUPT_STATEMENTS` times
      `STATEMENT_INSTRUCTIONS`, plus the instructions of one interrupt
      point: 36 (a 32-argument method call: 34 moves, the call, the
      result move) and 16 × 36 + 3 = 579. Some lines are not counted:
      a `function(` argument line of `__ulua_try`, a `nil` line, the
      jump at `else`. Each of them is a few instructions, and a
      statement near 36 instructions is a call, which ends the count.
      So an uncounted line cannot raise a gap above the bound; the
      largest measured gap is 49. A fitted constant (the measured maximum times 1.25) was
      false: a 32-argument call after 15 statements gave 49 against 44
      (P5.5 review 2).
    - A test runs shapes at the source ceiling under `lua_singlestep`
      and counts the instructions between two interrupts. It asserts
      two facts: the largest gap is at most `INTERRUPT_INSTRUCTIONS`,
      and the largest single statement is at most
      `STATEMENT_INSTRUCTIONS`. The shapes include, at the largest
      C15 operand counts, a call, a constructor, and a record literal
      after 15 counted statements; loop exits in the middle of a body
      (`break`, `continue`, the condition); and async modules: the
      async prelude, async functions, and the resume path of each
      `await`. The count is independent of time (principle 15).
    - The prelude follows the same rule: a run of more than
      `INTERRUPT_STATEMENTS` top-level prelude statements contains an
      interrupt point. The bodies of the fixed helpers are fixed text
      outside the count; their largest gap is measured (44 in
      `__ulua_sf`, P5.5 review 3) and stays below the bound. The async prelude gave a gap of 47 at its
      `__ulua_async_register` call (P5.5 review 2).
    - A ratio test runs each shape in `while (true)` at the default
      `max_steps` and asserts at most 6 times the run time of
      `while (true) { i++ }` (gate timing).
    - The production compile options stay at O1. A change of the
      options reruns that test: O2 can unroll the loop form.

## 6. Limits

| Limit | Applies to | Diagnostic or failure |
|---|---|---|
| `max_source_bytes` | `ulua_compile` | `limit:source-bytes` |
| `max_nesting_depth` | `ulua_compile`: parser guard and every compiler recursion | `limit:nesting-depth` |
| C15 bounds | `ulua_compile` | `limit:<name>` (collisions.md C15) |
| `max_memory_bytes` | each instance: the budget of its `lua_Alloc` | `limit:memory` |
| `max_steps` | each host-started run (`ulua_instance_new`, `ulua_call`, `ulua_call_start`, each settle of `ulua_pump`): interrupt callbacks | `limit:steps` |
| `max_open_requests` | each instance: open async host requests (`abi.md` §6.4); default 256 | `limit:requests` |
| Luau call depth | each call (`LUAI_MAXCALLS`, `LUAI_MAXCCALLS`) | `limit:stack` |

C15 bounds of P1, each a fixed compiler constant:

| Code | Bound | Value |
|---|---|---|
| `limit:parameters` | parameters of one function | 32 |
| `limit:arguments` | arguments of one call | 32 |
| `limit:template-parts` | literal parts and substitutions of one template literal, empty literal parts included | 1025 |
| `limit:temporaries` | temporaries, default-parameter copies, and generated locals of one generated function; they never spill (§5.8) | 136 |
| `limit:upvalues` | upvalues of one generated function (§5.14) | 192 |
| `limit:fields` | fields of one object type, or fields and methods of one class, plus the fields of each object type without an alias name (the type of an object literal) nested in it at any depth, through arrays, `T \| null`, and function types; an alias name ends the count. A nested type counts at each position where it appears (`{ a: c, b: c }` with a 600-field `c` counts 1,202); the compiler computes the count once per type and stops at the bound, in linear time. `luau-analyze` gives "Code is too complex to typecheck" for a Luau table type of 2,983 fields and passes at 2,982; three nested 1024-field literals (20,214 B) fail and two pass (P2.5 review 7) | 1024 |
| `limit:type-depth` | the longest chain of type links from one type: an object field, an array element, a member of `T \| null`, a function parameter or result. Alias names and anonymous types count alike. A group of mutually recursive aliases (a strongly connected component of the alias graph) counts as many steps as it has type links inside it (each alias, and each array, `T \| null`, and function type node on a path between its aliases); a ring of 160 direct record aliases passes `luau-analyze` and 170 fail, a nullable ring of 250 fails, and a ring of 130 under a chain of 47 fails (P2.5 review 9); a ring of 12 aliases with 45 nested arrays per link (1,562 B) fails (P2.5 review 10). The compiler computes the depth once per type, in linear time. `luau-analyze` gives "Internal recursion counter limit exceeded in TypeSimplifier::simplify" for a local whose type is a chain of 58 nested table types and passes at 57; a chain through `T \| null` fails at about 250 (P2.5 review 8) | 48 |
| `limit:output` | generated bytes of one module (§3.10) | `OUTPUT_FACTOR` × source bytes + prelude |

`limit:nesting-depth` also bounds the nesting of the generated Luau
below the Luau parser recursion limit of 1000. A bound that S3 finds
necessary and that this table does not name is a contract question.

0. Each limit diagnostic carries the source offset, line, and column
   of the form whose lowering reaches the limit, never of a later
   statement.
1. A zero limit is a zero limit. It is never "no limit".
2. The library has a ceiling for `max_source_bytes` and
   `max_nesting_depth`. A request above the ceiling uses the ceiling.
3. `max_steps` counts interrupt callbacks. A helper calls a Luau
   library function only if the call does O(1) work, or work in
   proportion to the bytes that it allocates; a search or a scan is a
   Luau loop. So no library call holds a step beyond a constant time
   plus its allocation charge. Each allocation also
   charges `ceil(bytes / ALLOCATION_STEP_BYTES)` steps. At the limit the
   allocator returns null and the run gives `limit:steps` with the
   message `Step limit exceeded`. A reallocation to an equal or smaller
   size is never charged and never fails: Luau requires this
   (`VM/src/lmem.cpp` of the pin). The charge
   bounds the time of one step by a constant, so `max_steps` bounds
   the run time. P1 measures `ALLOCATION_STEP_BYTES` (§7). The
   interrupt points of §5.19 bound the instructions between two
   interrupts by `INTERRUPT_INSTRUCTIONS`.
   - A GC step is outside the time of one step: one step of the Luau
     incremental collector traverses a whole table, and the atomic
     phase clears every weak table (`VM/src/lgc.cpp`). The work of
     the collector is in proportion to the allocation that drives
     it, and allocation is charged. So `max_steps` still bounds the
     total run time. Measured (P5.5 review, Release): the longest
     single step is 0.078 ms with 5 MB live and 1.37 ms with 34 MB
     live and a table of 3M slots; 20M steps take 0.88 to 1.04 s at
     each heap size.
4. Compiler memory is outside `max_memory_bytes`. The source ceiling
   bounds it: the peak Rust heap of one `ulua_compile` is at most
   `COMPILE_MEMORY_FACTOR` bytes per source byte plus
   `COMPILE_MEMORY_BASE` bytes (P5.4). The test calls `ulua_compile`
   through the C ABI under a counting global allocator, so the copy
   of the bytecode is counted. The count does not depend on time or
   machine load, so the assertion is a byte count (principle 15).
   - The factor: every growth shape at 16 KiB and at 64 KiB, and the
     nested empty block shape (`{` and `}` 44 deep, repeated). The
     factor is the largest peak per source byte, times 1.25.
   - The base: a set of small sources of at most 64 bytes, accepted
     and rejected (an empty source, `;`, `let a=1`, `class A{}`, and
     repetitions of each up to 64 bytes). The base is the largest
     peak of the set, times 1.25. The test asserts the bound for each
     small source and for each shape.
   - The test prints the largest ratio and the largest small peak.
   - Native `luau_compile` memory: the output of each growth shape at
     64 KiB is measured. The record names the largest peak and the
     largest peak per output byte; it follows `OUTPUT_FACTOR`
     (§3.10).
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
| Ceiling of `max_source_bytes` | 65,536 | compiler heap peak 93.1 MB at the ceiling (`spillD`, a fresh arrow per array literal over spilled bindings, Release, P2.2 review 2); Rust allocations 39.1 MB |
| `LOCAL_BUDGET` | 176 | Luau accepts 200 live locals |
| `ULUA_MIN_RUN_STACK` | 512 KiB | 130,544 B at `LUAI_MAXCCALLS` in Debug |
| Default `max_memory_bytes` | 64 MiB | minimum instance 401,816 B |
| Default `max_steps` | 1,000,000 | a test runs to the bound and one step past it |
| `OUTPUT_FACTOR` | 80, prelude 2,086 B (1,222 B at P1) | largest measured ratio 71.69 (a fresh arrow per array literal over spilled bindings, P2.2 review 2); the growth test tracks the densest shape of 97 constructs (63.14) |
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
- It reads the `grants` header of each entry (corpus.md §3) and passes
  one capability per named fixture of corpus.md §7 to `ulua_compile`.
  The fixtures write to the same output lines as `print`.

## 9. Fuzzing (P5.2, D29)

1. **Tool.** The workspace crate `ulua_fuzz` builds the binary
   `ulua-fuzz`. It runs one target for a fixed number of inputs from a
   seed (`--target <name> --seed <n> --inputs <n>`), or for a time box
   (`--seconds <n>`). The same seed and the same input count give the
   same inputs. Fuzzing runs on the sanitizer build (§2.6).
2. **Targets.**
   - `compiler`: source bytes into `ulua_compile`. The inputs are
     mutations of the corpus programs and of the programs of the
     random checks: byte flips, inserts, and deletes; token inserts,
     deletes, and swaps; a subtree copied in place of another;
     splices between two programs; nesting raised past the limits;
     invalid UTF-8 and lone surrogate escapes; sizes up to the source
     ceiling.
   - `run`: each accepted input of `compiler`, instantiated and with
     each export called, under small run limits, with granted fixture
     capabilities (sync and async) whose results the fuzzer chooses.
   - `abi`: random sequences of the functions of `abi.md` §7 over a
     pool of live objects, objects whose parent is freed (an instance
     after its module is freed, a module after its capability is
     freed), and consumed ids (a freed pointer itself is a host
     precondition, `abi.md` §7.1, so the tool never passes one), with
     valid and invalid arguments
     (null pointers, wrong `struct_size`, nonzero unused fields,
     invalid UTF-8, objects of another context, ids that never
     existed or were consumed), and calls from inside host callbacks.
   - `requests`: random sequences of async starts, completions,
     failures, pumps, task queries, discards, revocations, and frees.
3. **Invariants.** Each input is a failure if any of these holds:
   - a sanitizer report, a signal, or a Rust panic;
   - the status `ULUA_INTERNAL` or a diagnostic with the code
     `internal`;
   - a status or a code that `abi.md` does not allow for the call and
     its arguments;
   - an accepted input whose generated Luau the pinned `luau_compile`
     refuses, or (on a sample of one input in 64) whose generated Luau
     `luau-analyze -j1` rejects, or (on the same sample) that stock
     `tsc` rejects with a diagnostic outside invariant 3;
   - a count of `ulua_instance_requests` that differs from the model of
     the fuzzer;
   - a compile of a source of at least 1 KiB that takes more than
     1,000 times the median compile time per source byte of the inputs
     of at least 1 KiB in its run (a report for review, not a
     wall-clock gate assertion; principle 15). A smaller source is
     outside the metric: the fixed cost of the compiler thread
     dominates it.
   Each `tsc` and `luau-analyze` batch has a deadline of 30 seconds,
   and the supervisor gives each input a deadline of 30 seconds. A
   witness that passes its deadline is a resource limit of that
   witness (invariant 3), recorded with its input and not a failure;
   the other inputs of the batch are checked again one at a time. An
   input that passes its own deadline is a failure with the key
   `hang`. So `--seconds` always ends a run.
4. **Findings.** A failure writes its input, its seed, and its target
   to `target/fuzz/findings/` when it occurs, and the fuzzer reduces it
   to a minimal input that keeps the failure; the record states whether
   the reduction reached a 1-minimal input or stopped at its budget. Each finding becomes a direct test or
   a corpus entry before its fix lands.
5. **Gate.** `sh tools/gate.sh sanitize` runs `ulua-fuzz --smoke`:
   each target for a fixed input count with a fixed seed (a smoke run of at most 60 seconds
   per target). A Phase Review runs each target for 60 seconds on the
   sanitizer build with a fresh seed (not the smoke seed), the four at
   the same time; its value is new inputs, not length. The only defect
   that fuzzing found came from a 300-input trial, and 10-minute runs
   (about 357,000 inputs) found none, so no measurement supports a
   longer run. The review records the seed, the input count, the
   accepted count, and the findings.

## 10. Fault injection (P5.3)

1. **Build.** The test-only feature `fault-injection` adds a hook to
   the budget allocator of each instance and to the native allocation
   of the request table: the hook fails the allocation with index `k`
   of a run (counted from the start of the run) and no other. Neither
   library build has the hook.
2. **Sweep.** For each program of the sweep set (one corpus entry per
   P1 to P4 family, an async export with host requests, a module with
   a grant, and the example scripts), the test first counts the
   allocations `n` of an undisturbed run of `ulua_instance_new`, of
   each export call, of each completion, and of each settle. Then it
   runs once for each `k` from 1 to `n`, each in a fresh instance.
3. **Properties.** For each `k`:
   - the failed run gives `ULUA_FAILED` with `limit:memory`, or, where
     `abi.md` allows it for that call, the status of a refused
     resource (`ulua_request_complete`: `ULUA_FAILED`; §6.4.2); never
     `ULUA_INTERNAL`, a crash, or a sanitizer report;
   - no `catch` and no `finally` of the script runs after the failure;
   - the instance stays usable: a later call of a small export gives
     its result;
   - after `ulua_instance_free`, the bytes that the budget allocator
     counts as live return to zero, and the request table holds no
     record (the leak check that LeakSanitizer gives elsewhere).
4. **Rust allocations.** A Rust allocation failure aborts the process
   (the Rust standard library). The library does not recover from it;
   a host that needs that guarantee bounds the source size and the
   instance memory (§6). This is a host precondition, not a tested
   path.
5. **Cost.** The sweep runs in the sanitize gate when its count of runs
   is below 20,000 per program; a larger program uses every `k` that is
   a multiple of `ceil(n / 20000)`, and every `k` within 64 of a
   phase boundary (the start of `instance_new`, of each call, of each
   settle). The test states its cost (principle 17).
