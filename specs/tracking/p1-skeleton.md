# P1 vertical skeleton

Status: COMPLETE, 2026-10-12. Contracts: `pipeline.md` Rev 6, `abi.md` Rev 4,
`corpus.md` Rev 2, `surface.md` Rev 8, `collisions.md` Rev 19.

## Scope

- Corpus entries with `phase: P1`: 12 accept (`a01` to `a08`, `a10`,
  `a22`, `a25`, `a26`), 17 reject. Every other entry is rejected with
  the code `unsupported` (accept, trap) or with any code (reject).
- P1 forms: module-level function declarations; `let` and `const`;
  `number`, `boolean`, and `string` values; string literals and
  template literals; `print`; `String(n)`; the number and boolean
  operators of surface.md §3; `if`, `while`, `for`, `break`,
  `continue`, `return`; the `?:` operator; parameter defaults.
- Later phases: nested functions and arrows, arrays, `null`, object
  types, classes, string operations and `Utf8`.

## Slices

| Slice | Content | Agent |
|---|---|---|
| S1 | Workspace, `build.rs`, shim, C ABI, `ulua-corpus`, and a compiler that accepts `a01` | Codex |
| S2 | The checker and the emitter for every P1 form; every P1 entry passes | Codex |
| S3 | The measurements of `pipeline.md` §7, the C15 bounds, and their tests at and past each value | Codex |
| S4 | Phase Review round 1 fixes: `pipeline.md` Rev 2, `abi.md` Rev 1 | Codex |
| S5 | Phase Review round 2 fixes: `pipeline.md` Rev 3, `abi.md` Rev 2 | Codex |
| S6 | Phase Review round 3 fixes: `pipeline.md` Rev 5, `collisions.md` Rev 13 C7 | Codex |
| S7 | Phase Review round 4 fixes: `collisions.md` Rev 14 C7, `surface.md` Rev 6 | Codex |
| S8 | Phase Review round 5 fixes: `collisions.md` Rev 17 C7 rule 4, `surface.md` Rev 8 | Codex |
| S9 | Phase Review round 6 fixes: `collisions.md` Rev 18 C7 rule 4 and C11 | Codex |
| S10 | Phase Review round 7 fixes: `collisions.md` Rev 19 `ALIAS_TEST_BOUND` | Codex |
| S11 | Phase Review round 8 fixes: `pipeline.md` Rev 6 §4.6a, `abi.md` Rev 4 §3 | Codex |

## Exit checklist

- [x] `sh tools/gate.sh full` passes with the Luau witnesses.
- [x] Every function of `abi.md` has a direct unit test; the layout test
  passes (round 9: `crates/ulua/src/tests.rs`, `header_layout`).
- [x] `pipeline.md` §7 values measured and pinned by tests.
- [x] P0 carry-over: the P1 Phase Review verifies C12.6 and C16.4 (`plan.md` §9 Rev 2).
  Round 9 verified both by the contract text: C12.6 agrees with the
  JavaScript order of `push`; C16.4 is restrictive against `tsc` and
  sound for JavaScript. P1 rejects their forms with `unsupported`, so
  the executable check moves to the P2 exit (`plan.md` §9 Rev 4).
- [x] P0 carry-over N3 (`a25`), N4 (`a26`), N5 (surface.md Rev 5): `a25`
  and `a26` pass; N5 has the P2 entry `a27`.
- [x] Phase Review with no open CRITICAL or MAJOR finding (round 9).

## S1 record

- Codex task; all Cargo commands offline. 19 tests. `gate.sh full`:
  every step passes except `luau-witnesses`, which fails on the 28
  P1 entries of S2 only. a01 passes; 69 later-phase entries are
  rejected; no `internal`.
- Contract answers during S1: the runner emits witness Luau through
  `ulua_compiler` (pipeline.md §8); crates.io `oxc_index 5.0.0` stays
  as the fork states (pipeline.md §1); codes `host:output` and
  `internal` (abi.md §3).
- Provisional constants (S3 measures): compiler stack 8 MiB; source
  ceiling 1 MiB; nesting ceiling 128; minimum run stack 4 MiB; default
  instance memory 64 MiB; default steps 1,000,000.

## S2 record

- Codex task. Tests: ulua 18, ulua_compiler 21, ulua_corpus 1.
  `gate.sh full` passes in full: 29 P1 entries pass; 69 later-phase
  entries are rejected; 12 generated modules pass `luau-analyze`.
- Restrictive readings by the implementation: in a declaration with
  several declarators, no binding is visible until the whole
  declaration ends (C10); string `+`, `+=`, and members stay
  `unsupported` in P1; `===` accepts two operands of one primitive
  type (C7); `undefined` as a binding name is rejected (C2).
- Provisional: `LOCAL_BUDGET = 160` (S3 measures).
- Emission form: each operand goes through a typed temporary that the
  function declares at its entry (C12). P6 measures its cost.

## S3 record

- Codex task. Tests: 41 in the default build, 42 with the
  `measurements` feature. `gate.sh full` passes. The added gate test
  `crates/ulua/tests/s3_limits.rs` costs 1.75 s in Debug, 0.52 s in
  Release.
- Opt-in reproduction: `cargo run -p ulua --features measurements
  --example s3_measure -- MODE`, MODE = `stack`, `heap`, `native`, or
  `run`. Add `--release` for Release. The feature adds the probe
  `native/probes/s3.cpp` and the entries `measure_compile` and
  `measure_template`. No default build contains them.
- The shim maps the error text `not enough memory` to `limit:memory`
  and collects after it.
- Contract answers: a template of `n` substitutions has `2n+1` parts, so
  the input past 1025 parts has 1027 parts. `LOCAL_BUDGET+1` bindings
  spill and compile (§5.8). P1 source cannot reach recursive protected
  C calls, so that bound uses a native fixture of the measurement
  build.

## Measurements

Compiler stack, bytes per level (parse/check/emit), slope over levels
8 to 16. "Accepted" is the largest repetition count at depth ceiling
48; one more gives `limit:nesting-depth`.

| Form | Accepted | Debug | Release |
|---|---:|---|---|
| Parentheses | 42 | 3344/15952/0 | 1728/0/0 |
| Unary `!`, `-` | 42 | 368/15952/8720 | 112/1344/760 |
| `?:` | 42 | 720/15952/8672 | 688/1344/746 |
| Nested `if` | 15 | 1664/20272/23408 | 688/2998/1408 |
| `else if` | 43 | 752/9568/11402 | 288/1424/704 |
| Blocks | 23 | 1008/11056/12032 | 400/1744/704 |
| `while` | 15 | 1616/20272/23408 | 688/2998/1408 |
| `for` | 15 | 3440/20272/23408 | 896/2992/1408 |
| Calls | 42 | 3040/15953/8720 | 1472/1344/766 |
| Flat `+` chain | 42 | 0/15952/8720 | 0/1344/760 |
| Template nesting | 42 | 3056/15952/8720 | 1856/1344/766 |
| Assignment chain | 41, then `C11` | 832/0/- | 688/0/- |

| Value | Measurement | P1 constant |
|---|---|---|
| Compiler stack | Debug peak 716,864 B; Release 88,064 B | 2 MiB; Debug passes with 1 MiB |
| Source ceiling | Release at 65,536 B: 26 to 69 ms per input kind, except `functions` 502 ms | 65,536 B |
| Compiler heap | Release at the ceiling: native peak 32,525,632 B (`flat`); Rust peak 22,522,853 B | ceiling bounds it |
| Parameters | Luau: 200 plain; 100 with default copies | 32 |
| Arguments | Luau: 78 with 176 locals | 32 |
| Non-spill locals | Luau: 168 with 32 parameters and default copies | `TEMPORARY_BOUND` 136 |
| Local budget | Luau: 200 live locals | 176 |
| Template parts | Luau accepts 200,003 parts from the production emitter | 1025; no failure found |
| Host stack | protected C calls at `LUAI_MAXCCALLS`: 130,544 B Debug, 127,968 B Release | 512 KiB |
| Instance memory | minimum instance 401,816 B; one byte less fails; 64 MiB admits 24 string doublings, not 25 | 64 MiB |
| Steps | 1,000,000 callbacks pass; the next fails | 1,000,000 |

Native heap peaks come from a 100 µs sampler. Rust allocation counts
are exact.

## S4 record

- Codex task. Tests: 49 in the default build, 51 with all features.
  `gate.sh full` passes; `luau-witnesses` takes 20 s, of which the 7
  stress modules take about 18 s of `luau-analyze`.
- Typed writes: the bytecode of the 7 stress modules is the same with
  and without the write assertions.
- Linear time (`crates/ulua/tests/s4_growth.rs`, Debug, minimum of 3
  runs, test cost 3.4 s): the ratio of 64 KiB to 16 KiB time is 3.71
  to 3.96 for all 11 shapes (`bindings`, `dense`, `functions`,
  `parameter-functions`, `expressions`, `templates`, `unary`, `flat`,
  forward and backward call chains, star). Before the fix, `functions`
  had a ratio of about 3.6 per doubling.
- `ALLOCATION_STEP_BYTES` = 256 (Release, `s4_measure`): a tight loop
  costs 15.3 ns per step. The slowest allocation input is
  `String(i*1e280)`, 7.78 ns per allocated byte, so 1,000,000 charged
  steps cost about 2.0 s. The review input (four-part template of a
  2^18-byte string) stops at `limit:steps` after 0.055 s at the
  defaults.
- Stress modules, 65,536 bytes each, `luau-analyze` time: `&&` 12.6 s,
  `||` 12.7 s, `?:` 12.9 s, `if` 14.3 s, `while` 7.6 s, `for` 0.5 s,
  `+=` 0.4 s; all 7 in one process 18.3 s; exit 0.
- Depth grid: 109 pairs of an accepted input and an input one level
  past it; each rejection has a non-zero offset.
- The 513-part template runs within the default memory. The
  `limit:memory` failure of the review did not reproduce on the
  binary before the fix.

## S5 record

- Codex task. Tests: 54 in the default build, 56 with all features.
  `gate.sh full` passes: `cargo-test` 18 s (the growth test costs
  11.0 s in Debug), `luau-witnesses` 22 s (11 stress modules, 18.6 s
  of `luau-analyze` in one process).
- Spill candidates at 65,536 bytes (5,549 bindings in a function and in
  a module chunk; 2,989 short empty functions). One constructor fails
  all three ("Code is too complex"); a type alias with a constructor
  fails all three; one table with `{} :: any` passes. Chosen: tables of
  at most 128 fields with `{} :: any`: 0.354 / 0.077 / 0.095 s. A
  denser fixture of 11,510 bindings passes in a function and in a
  module (90 tables). One table of 20,000 fields passes in 6.7 s.
- Join candidates. Input: 3,609 outer bindings written inside 40
  nested `if`. An immediate live join at each level gives 144,360 join
  writes, 12,347,415 bytes (ratio 188), analyzer 4.4 s. The deferred
  join gives 1,760,664 bytes (ratio 26.9), analyzer 0.4 s. The review
  input (41 nested `if` around 5,000 declarations) goes from
  35,107,611 to 648,288 bytes. A join emitted inside the next branch,
  and no join at all, each make the S4 `if` stress module run past
  40 s.
- Function types in write assertions: 32 default parameters, repeated
  calls: the expanded signature gives 6,667,065 bytes and an analyzer
  failure; a type alias gives 1,454,408 bytes and exit 0.
- `OUTPUT_FACTOR` = 64, prelude 843 bytes. Largest measured ratio over
  17 shapes at the ceiling: 41.7 (`flat`). Growth ratios 4n/n: 3.45 to
  3.95.
- Compiler heap at 65,536 bytes, Release, 17 shapes: total peak
  51,883,760 B (`flat`); Rust requested peak 40,619,549 B
  (`default-calls`). Native values come from a 100 µs sampler.
- Stress admission: each of the 11 kept shapes fails without its form.
  `&&`, `||`, `?:`, `if`, `while` without typed writes: "Internal
  recursion counter limit exceeded". Function, module, nested, nested
  writes, and short-function bindings with one spill constructor, and
  default calls with expanded signatures: "Code is too complex to
  typecheck". `for` and `+=` passed without the form and left the set.
- Allocator: an equal or smaller reallocation is not charged and does
  not fail. A native fixture runs `luaD_pcall -> restore_stack_limit
  -> luaD_reallocCI` with the steps used up; the shrink succeeds.
- Exported `ulua_` symbols of the static library: the 14 functions of
  abi.md §7 and 5 `ulua_internal_shim_*`. The layout probe is in a
  test-only archive.

## S6 record

- Codex task. Tests: 59 in the default build, 61 with all features.
  `gate.sh full` passes: `cargo-test` 22 s, `luau-witnesses` 21 s.
- Pending joins: one depth bit set per binding; no set copy or merge.
  `ir::reads` and `ir::writes` no longer build sets per statement.
  Release, 16 KiB / 64 KiB: `pending-if` 34.6 / 1,029.2 ms (ratio
  29.7) before, 7.6 / 28.9 ms (3.79) after; `pending-loops` 18.6 /
  476.1 ms (25.6) before, 6.4 / 24.4 ms (3.80) after.
- `OUTPUT_FACTOR` = 80. Largest ratio over 21 shapes: 56.71
  (`spilled-and`). The `1e308` shape drops from 64.72 to 15.91 with
  shortest number literals.
- Spill: typed constructors of at most 128 fields. Binding stress
  modules pass in 0.19 to 1.03 s; one constructor fails each in 0.46
  to 3.33 s ("Code is too complex to typecheck").
- C7: `-0 === 0` and `0 === -0` literals: `tsc` accepts, `node` gives
  `true`. The checker treats them as one value.
- Gap found by S6: `(!true) === true` is accepted; `tsc` gives TS2367.
  Primary measurement of 14 forms (`tsc` 5.9.2, prelude, `--strict`).
  Each is the return expression of `function f(c: boolean, p: boolean):
  boolean { const a = 1; return <expr>; }`:

  | Expression | `tsc` |
  |---|---|
  | `(!true) === true` | TS2367 |
  | `(!true) === false` | accepts |
  | `(c ? 1 : 2) === 3` | TS2367 |
  | `(c ? 1 : 2) === 2` | accepts |
  | `(p && true) === false` | accepts |
  | `(true && false) === true` | TS2367 |
  | `(p \|\| true) === false` | TS2367 |
  | `` `a` === "b" `` | TS2367 |
  | `(-a) === 1` | accepts |
  | `a === -1` | TS2367 |
  | `(!p) === true` | accepts |
  | `(c ? true : false) === true` | accepts |
  | `(c ? "x" : "y") === "z"` | TS2367 |
  | `(1 + 1) === 3` | accepts |

  Codex measured two more: `(false && true) === true` and `(true ||
  false) === false` give TS2367. The right operand enters the result
  only if the left operand does not short-circuit.
  C7 now states the literal-set model; `r72` and `a28` pin it.

## S7 record

- Codex task. The literal-set model is removed. C7 Rev 14 and Rev 15:
  a structural operand check and a scoped undo log for tested
  references. The return rule tells `(true)` from `true`.
  `gate.sh full` passes: `cargo-test` 15 s, `luau-witnesses` 22 s.
- Random equality programs: 2,400 generated; 465 accepted, each passes
  `tsc`; 1,751 rejected with C7, 184 with `unsupported`.
- The 15 narrowing forms of round 4 (bodies of `function f(p: boolean,
  n: number, s: string): boolean`): each gives C7, and `tsc` gives
  TS2367 for each (primary measurement).
- Growth: left-nested `%` chains over a spilled `x`, 36 to 42 terms,
  ratio 62.36 to 62.67 at 65,536 bytes. Largest time ratio over 28
  shapes: 6.61 (limit 8). A 25% margin needs `OUTPUT_FACTOR` 78.34;
  80 stays.
- A test builds an oversized IR and gets `internal` from the output
  check.
- Debug costs: growth test 6.84 s, bytecode test 3.21 s; each file
  states its cost.

## S8 record

- Codex task. C7 rule 4 as a flow fact: a join takes the intersection
  of the incoming edges; each edge record costs O(1); a shared log is
  read once per join; depth is at most 48. h1 to h5, `r76`, `r77`
  give C7; `r78` and the two reviewer `let` forms give C11.
- Random P1 programs: 6,000 generated; 4,517 accepted, 579 rejected
  with C7, 904 with C11. Accepted programs: 0 `tsc` errors, 0
  `luau-analyze` errors, 0 `node` differences.
- Growth test: minimum of 2 runs per size for ratios of 5 or more;
  Debug 7.75 to 7.87 s; largest time ratio 6.49, largest output ratio
  62.67.
- `gate.sh full` passes: `cargo-test` 16 s, `luau-witnesses` 21 s.

## S9 record

- Codex task. Aliased boolean conditions carry the sets of their
  initializers; literal `true` and `false` conditions keep one edge;
  `checker/strict.rs` gives C11 for strict-mode early errors. `r79`,
  `r80` give C7; `r81`, `r82` give C11; `a10` passes.
- Random P1 programs: 5,000 generated; 3,701 accepted, 1,299 rejected
  with C7. Accepted programs: 0 `tsc` errors, 0 `tsc` resource limits,
  0 `luau-analyze` errors, 0 `node` differences.
- Join growth shapes, 4x ratio Debug / Release: deep 3.76 / 3.82, wide
  3.92 / 3.89, nested loops 3.81 / 3.85. Growth gate: Debug 7.06 s,
  Release 2.05 s; largest ratio 6.49 / 6.42.

## S10 record

- Codex task. Alias sets are flattened and bounded by
  `ALIAS_TEST_BOUND` = 8 (`bounds.rs`); 9 references give C7 at the
  declaration (`r83`). A hashbang line has its own C11 message.
- Growth, 8 to 32 KiB: alias chain Debug 11.66 to 45.58 ms (3.91),
  Release 3.75; an alias with 8 references Debug 3.93, Release 3.78.
  Growth gate: Debug 7.77 s, largest ratio 6.52; Release 2.14 s.
- Random P1 programs: 5,000 generated; 1,955 accepted, 3,045 rejected
  with C7; accepted programs: 0 `tsc` errors, 0 resource limits, 0
  `luau-analyze` errors, 0 `node` differences. The longest accepted
  alias chain has 76 links.

## S11 record

- Codex task. `native/shim/runtime_errors.h` lists the size and depth
  errors of the pinned VM, each with its `VM/src` location.
  `limit:memory`: not enough memory, string length overflow, memory
  allocation error: block too big, table overflow, buffer too large,
  resulting string too large, string slice too long, too many
  captures, size specifier is too large, and the `packsize` and
  `move` argument errors. `limit:stack`: stack overflow and its
  variants, C stack overflow, stack limit, too many results or
  arguments, pattern too complex, `__index` and `__newindex` chain too
  long. `LUA_ERRMEM` is memory; a depth `LUA_ERRERR` is stack. Other
  run errors stay `internal`.
- The two review cases: before, status 6 then 7; after, status 3
  `limit:memory` then 0. The large-budget tests run only with the
  `measurements` feature (0.80 s Debug).
- Run messages carry no generated-Luau position.
- Compiler heap at 65,536 bytes, Release, 33 shapes: total peak
  58,957,600 B and native peak 50,762,190 B (`spilled-mod-39`); Rust
  peak 40,619,549 B (`default-calls`).
- Tests: 86 with all features. `gate.sh full`: `cargo-test` 12 s,
  `luau-witnesses` 21 s.

## Phase Review round 1

Reviewer: fresh agent, diff `f5f0bf1..dc36b3b`. 4 MAJOR, 7 MINOR.

| Finding | Severity | Resolution |
|---|---|---|
| A `string` export compiles; a call loses the result or poisons the context | MAJOR | abi.md §4.3: `unsupported` at compile; S4 |
| `b = b && b;` x100 (about 700 bytes) fails `luau-analyze`: "Code is too complex to typecheck" | MAJOR | pipeline.md §5.10, §5.11; S4 |
| The emitter is quadratic in functions x bindings: emit 15/38/99/357 ms at 8/16/32/64 KiB | MAJOR | pipeline.md §3.10; S4 |
| `max_steps` does not bound time: a 4-part template of a 2^18-byte string costs 4.5 ms per step, 75 min at the defaults | MAJOR | pipeline.md §6.3: allocation charge; S4 |
| A template lowers to one `..` per part: quadratic copy | MINOR | pipeline.md §5.12; S4 |
| Sequential `for` loops each keep a generated local: 68 loops give `limit:temporaries` | MINOR | Gap; P6 |
| Emitter limit diagnostics have offset 0; the emitter counts 2 levels per statement, the checker 1 | MINOR | pipeline.md §3.4; S4 |
| The `limit:temporaries` row names parameters; the code does not count them | MINOR | Contract text corrected |
| N5 has no accept entry | MINOR | `a27-parameter-callee`, phase P2 |
| A refused compiler thread poisons the context | MINOR | abi.md §3: `host:resources`; S4 |
| The layout test does not check status and tag values | MINOR | abi.md §8; S4 |

Analyzer evidence (luau-analyze of the pin, hand-written Luau, one
function, N repetitions). "Fails" means "Code is too complex to
typecheck" or "Internal recursion counter limit exceeded":

| Form | N=100 | N=1000 |
|---|---|---|
| `y = x; if b then y = x + 1 end; x = y` | fails | fails |
| `t1 = b; t0 = t1; t0 = if t0 then b else false; b = t0` | fails | fails |
| The same two forms, each write `:: T` | passes | passes, 0.1 s |
| `if b then x = x + 1 end` | passes | passes |

## Phase Review round 2

Reviewer: fresh agent, diff `f5f0bf1..1db351e`. 3 MAJOR, 4 MINOR.

| Finding | Severity | Resolution |
|---|---|---|
| The spill table constructor fails `luau-analyze` at 3200 `let` in one function (37,351 bytes) and at 3300 module bindings | MAJOR | pipeline.md §5.8, §5.11; S5 |
| Join writes grow with depth x bindings: 41 nested `if` around 5,000 bindings (65,536 bytes) give 38 MB of Luau and 360 MB RSS at compile | MAJOR | pipeline.md §3.10, §5.10; S5 |
| The step charge can fail a shrinking reallocation; Luau requires that it never fails (`lmem.cpp`); `luaD_reallocCI` after a call-depth overflow runs outside protection | MAJOR | pipeline.md §6.3; S5 |
| The `for` and `+=` stress modules pass without the write assertions: 0.9 s of gate time without a reason | MINOR | pipeline.md §5.11; S5 |
| `ALLOCATION_STEP_BYTES` is defined in Rust and in C; only a feature build compares them | MINOR | abi.md §8; S5 |
| `ulua_layout_probe` and `ulua_shim_*` are exported from the static library | MINOR | abi.md §1.10; S5 |
| `limit:steps` from the allocator has the message `not enough memory` | MINOR | pipeline.md §6.3; S5 |

Primary measurement: the S4 `if` stress module with the join writes
removed (50,405 to 45,368 lines) did not finish `luau-analyze` in 5
minutes. The join writes are necessary; their count is the defect.

## Phase Review round 3

Reviewer: fresh agent, diff `f5f0bf1..2354864`. 2 MAJOR, 3 MINOR.

| Finding | Severity | Resolution |
|---|---|---|
| Deferred joins copy and merge the pending set per compound statement: 16 KiB 37 ms, 64 KiB 1,080 ms (ratio 29), Debug 11.2 s | MAJOR | pipeline.md §5.10; S6 |
| The checker accepts `tsc`-rejected equalities (TS2367): `1 === 2`, `const a=1; const b=2; a===b`, `let x: boolean = true; x === false`. Inline literal operands also fail `luau-analyze` (17 of 660 random programs) | MAJOR | collisions.md C7; `r70`, `r71`, `a28`; S6 |
| `OUTPUT_FACTOR` is not enforced. `1e308` emits 309 digits: ratio 64.79. `&&` over spilled bindings: 62.3 | MINOR | pipeline.md §3.10, §5.13; S6 |
| The `{} :: any` seed is not necessary: typed constructors of at most 128 fields pass all five binding stress modules | MINOR | pipeline.md §5.8; S6 |
| abi.md §1.10 is false for the static library: it exports about 1,866 Luau C++ symbols | MINOR | abi.md §1.10 names the dynamic library |

Primary measurement (`tsc` 5.9.2, prelude): `p = true; return p ===
q;` with `q: boolean`, `p === true` with an unassigned parameter, and
`let x = n > 1; x === g(true)` give no error. `tsc` gives TS2367 only
when both operands narrow to literal types. `a28` pins the accepted
forms.

## Phase Review round 4

Reviewer: fresh agent, diff `f5f0bf1..0f84500`. 4 MAJOR, 4 MINOR.

| Finding | Severity | Resolution |
|---|---|---|
| C7 ignores `tsc` narrowing by conditions: `if (p) { return p === false; }`, `return n === 1 && n === 2;`, `if (n !== 1) { return true; } return n === 2;` and 12 more give TS2367 and compile | MAJOR | D15; C7 Rev 14 rules 3 and 4; `r72` |
| A template with literal substitutions is *wide*: `` `${1}` === "2" `` gives TS2367 and compiles | MAJOR | C7 Rev 14 rule 3; `r74` |
| `n === NaN` compiles; `tsc` gives TS2845 | MAJOR | C7 Rev 14 rule 2; `r73` |
| `while ((true)) { return 1; }` ends a function with a result; `tsc` gives TS2366 | MAJOR | surface.md Rev 6; `r75` |
| `x=x%x%...%x` over a spilled `x` gives ratio 63.8; margin to 80 is 0.25 over 25% | MINOR | S7 adds the shape |
| No test builds an output-bound violation | MINOR | S7 |
| The C7 text and the code disagree on a *narrowable* condition | MINOR | Closed by C7 Rev 14 |
| Stale gate cost statements: `s4_growth.rs` states 12 s, measures 18.9 s; `s4_bytecode.rs` states none, measures 3.7 s | MINOR | S7 |

Round 3 and round 4 found 1 and 3 new instances of the class "the
checker accepts a `tsc`-rejected program". The user chose a
restrictive structural subset (D15). C7 Rev 14 replaces the
literal-set model: no `boolean` equality operands, no `NaN` operand,
and one *wide* operand that `tsc` cannot type as a literal.

Corpus changes for C7 Rev 14: `a10` replaces `t === f` and `t !== f`
with `?:` forms and declares `let zero`; `a20` (P2) compares `let`
bindings with literals; `a28` becomes `a28-equality-operands`; `r70`
and `r71` change messages; `r72` becomes
`r72-tested-reference-equality`; new `r73`, `r74`, `r75`. The goldens
of `a10` and `a20` do not change.

C7 Rev 15 (user request under D15): `==` and `!=` follow `===` and
`!==`. `tsc` 5.9.2: `n == s`, `n == b`, `s == b`, `n != s` give TS2367;
`n == 3`, `s == "a"`, `b == true`, `n == null` give no error. `node`:
69 same-type pairs of `0`, `-0`, `1`, `NaN`, `Infinity`, `-Infinity`,
`0.1`, `""`, `"a"`, `"0"`, `"NaN"`, `true`, `false` give the same
result for `==` and `===`. `r17` now fails rule 3; new `a29`.

Invariant 5 revision (user decision): `node` is a divergence detector.
A decided divergence is named in `collisions.md` with the measured
outputs of both sides, and its entries state `js-comparable: no <id>`.
corpus.md Rev 2 adds the header; every current accept and trap entry
states `js-comparable: yes`. P2 can now decide `x === null` and library
misses as named divergences instead of rejections.

Invariant 5 audit (fresh agent, read-only, measured with `node` and
the Luau CLI). No P1 rule changes. Stale text fixed: CLAUDE.md
non-goals and principle 3, plan.md §2, §3, §5, D3 (§9 Rev 3),
surface.md intro and the `==` row (it contradicted C7 Rev 15),
collisions.md preamble and C5.5. Candidates for later phases, each
with an agreeing lowering unless noted: C3 truth tests of `number` and
`string` (`n ~= 0 and n == n`, `s ~= ""`); C4.6 `string + number` with
the C4.5 function; C4.4 `**` with a guard for base 1 or -1 and a
non-finite exponent (`1**NaN`: `node` NaN, Luau `^` 1); C4.7 bitwise
with a signed fix-up (`-1|0`: `node` -1, `bit32.bor` 4294967295);
`trim` with the JavaScript whitespace set; `x?: T` as a checked
`T | undefined`. The one clean divergence candidate: string `<` and
`>` (Luau compares bytes; they differ from `node` only for astral
characters against U+E000 to U+FFFF). Kept rejections, each silently
wrong otherwise: string `length` and index (byte vs UTF-16), case
mapping, function `===`, `T | null` defaults, C9.3.

P2 note on `undefined` (D16): `undefined` is rejected. No accepted
form and no library call gives `undefined` under `node`. A library
call whose miss is `undefined` under `node` is not in the library; the
library offers a presence test plus a call with a fallback, or a trap.
P2 states the consequence for `x === null` in C2.

P2 note: rule 3 lists no member read and no object operand. `a21`
(`other.end === this.end`, `a === b` on objects) needs a P2 revision
of rule 3.

## Phase Review round 5

Reviewer: fresh agent, diff `f5f0bf1..0e681f1`. 3 MAJOR, 4 MINOR.
Random programs: 26,500 generated, about 3,800 accepted; 0
`luau-analyze` errors, 0 `node` mismatches, no `internal`; `tsc` errors
only from findings 1 and 2.

| Finding | Severity | Resolution |
|---|---|---|
| C7 tests do not leave their region: a test inside a closed block, a branch whose sibling exits, `while (true)` with `break`, and the `for` update; each compiles and gives TS2367 | MAJOR | C7 rule 4 as a flow fact (Rev 17); `r76`, `r77`; S8 |
| Truthiness narrowing gives a `let` a literal type: `if (b) { let v = b; v = false; }` gives TS2322 | MAJOR | surface.md Rev 8: unannotated boolean `let`; `r78`; S8 |
| 1,000 `if` in one function (14.1 KB) give TS2563; a 64,983-byte loop input overflows the `tsc` stack; the compiler accepts both | MAJOR | Owner decision: `tsc` resource limits are outside invariant 3 (CLAUDE.md) |
| surface.md `==` row contradicts C7 Rev 15 | MINOR | Fixed in surface.md Rev 7 |
| `-(1)` is wide in the C7 text, not in the code | MINOR | C7 text: `e` without its parentheses |
| The return rule in the code accepts a block and statements after `return`; the text lists neither | MINOR | surface.md Rev 8 states the statement-list rule |
| The growth test changed to 8/32 KiB, one run; the S4 record still says 16/64 KiB, minimum of 3. `spilled-mod` ratios 6.25 to 6.40 against 8 | MINOR | Recorded here; S8 reviews the margin |

## Phase Review round 6

Reviewer: fresh agent, diff `f5f0bf1..317fba6`. 3 MAJOR, 3 MINOR.
Random programs: 13,500 generated, 5,328 accepted; `tsc` errors only
from findings 1 and 2; 0 `node` mismatches; `luau-analyze` exit 0.

| Finding | Severity | Resolution |
|---|---|---|
| Aliased conditions: `const isOne = x === 1; if (isOne) { return x === 2; }` and 6 more compile; `tsc` gives TS2367 | MAJOR | C7 rule 4: a boolean binding read has the sets of its initializer; `r79`; S9 |
| A bare `true` or `false` in `if`, `?:`, `&&`, `||` keeps both edges; `tsc` drops one: `if (x !== 1) { if (true) { return false; } } if (x === 2)` compiles, TS2367 | MAJOR | C7 rule 4: literal conditions have one edge; `r80`; S9 |
| Strict-mode early errors pass: `010` (prints 8), `08`, `"\01"`, `"\8"`, `let eval`, `let arguments`, `let yield`, `let await`, and other reserved words; `node` gives SyntaxError, `tsc` TS1121/1214/1215/1487/1488/1489 | MAJOR | C11 strict-mode rule; `r81`, `r82`; S9 |
| The C7 text does not state that a `while (true)` test has no false edge | MINOR | Closed by the literal-condition rule |
| The boolean `let` rule accepts parentheses; the text does not say so | MINOR | surface.md text |
| No gate measures the join cost: deep, wide, and nested-loop join shapes measure 3.68 to 3.74 per 4x (Release) | MINOR | S9 adds the three shapes to the growth test |

## Phase Review round 7

Reviewer: fresh agent, diff `f5f0bf1..4dd83ea`. 1 MAJOR, 1 MINOR.
About 1,050 accepted programs (854 random, about 200 hand-written edge
cases over literals, escapes, strict mode, declarations, shadowing,
control flow): each passes `tsc`, runs under `node` with the same
output, and passes `luau-analyze`.

| Finding | Severity | Resolution |
|---|---|---|
| The alias walk is quadratic: `const aN = aN-1 \|\| x === 1;` chains, read many times; Debug 8 KiB 0.09 s, 32 KiB 1.00 s (ratio 11.1, limit 8); Release 16 KiB 0.04 s, 64 KiB 0.39 s | MAJOR | C7: flattened alias sets of at most 8 references (`ALIAS_TEST_BOUND`); `r83`; S10 adds an alias-chain growth shape |
| C11 states wrong `tsc` codes for `await`: TS1262 at module level, none in a function body | MINOR | C11 text |
| A hashbang line gives C11 with the message for module directives | note | S10 fixes the message |

## Phase Review round 8

Reviewer: fresh agent, whole phase, diff `f5f0bf1..cce8946`. 1 MAJOR,
3 MINOR. Random: 5,200 programs, 3,292 accepted; 0 `tsc` errors, 0
`node` differences, 0 `luau-analyze` errors, 0 `internal`. About 100
hand-written edge cases agree. Limits recover; busy rules hold; the
dynamic library exports the 14 functions only.

| Finding | Severity | Resolution |
|---|---|---|
| String doubling with `max_memory_bytes` 2 GiB and 10,000,000 steps gives `internal` ("string length overflow", `MAXSSIZE` 1 << 30) and poisons the context; a 17-part template at 4 GiB gives "memory allocation error: block too big" | MAJOR | pipeline.md §4.6a; S11 |
| The `OUTPUT_FACTOR` row of §7 states 56.7; S7 measured 62.7. The heap peak covers only the S5 shapes | MINOR | §7 row; S11 remeasures the heap |
| Run messages hold generated Luau lines (`script:46: stack overflow`) | MINOR | abi.md §3; S11 |
| `s4_review.rs` and `s5_native.rs` state no cost (0.13 s, 0.12 s) | MINOR | S11 |

## Phase Review round 9

Reviewer: fresh agent, whole phase, diff `f5f0bf1..87d38f4`. No
CRITICAL, no MAJOR, 4 MINOR. Extreme host limits (0 to UINT64_MAX for
memory and steps, a 512 KiB host stack) give only limit codes; no
`internal`, no poison, no crash. About 140 edge cases, 600 random
programs, 2,350 mutants, and 9,000 number formats agree with `tsc`,
`node`, and `luau-analyze`. A depth sweep of 32,500 compiles gives no
`internal`.

| Finding | Severity | Resolution |
|---|---|---|
| `runtime_errors.h` maps the `buffer.writestring` argument error "string length overflow" (`VM/src/lbuflib.cpp:228`) to `limit:memory`; the text equals the `MAXSSIZE` error | MINOR | Not reachable in P1 (no `buffer` library). Gap for the phase that exposes `buffer` |
| The two size errors that P1 reaches are tested only with the `measurements` feature | MINOR | Default-build test in `s8_runtime.rs` through the native fixture (1.08 ms). It reaches "string length overflow" through the `buffer.writestring` argument path, so it checks the list entry, not the `MAXSSIZE` path |
| A `%` chain after 180 `let` bindings peaks at 61.1 MB, above the recorded 59.0 MB | MINOR | `pipeline.md` §7 row |
| The exit checklist leaves C12.6, C16.4, N5 and the ABI test item open | MINOR | Checklist closed with the evidence above |

## Phase close

- `sh tools/gate.sh full` passes: 29 accept, 83 reject, 3 trap; 45
  entries in phase P1 or earlier; 14 generated modules and 11 stress
  modules pass `luau-analyze`; `cargo-test` 12 s; `luau-witnesses`
  22 s. `sh tools/hygiene.sh` passes.
- Running tasks at the close: none. Each Codex task and each waiter
  shell completed; the round 9 reviewer was stopped after its report.

## Failures before the fix

## Gaps

- `runtime_errors.h` classifies by message text. The `buffer` library
  raises "string length overflow" for an argument error. The phase that
  exposes `buffer` classifies by source location, not by text.

- Sequential `for` loops each keep one generated local (round 1
  MINOR). P6 measures the reuse of locals across sibling scopes.

- The `functions` input of `M heap` takes 502 ms in Release at 65,536
  bytes. The other inputs take at most 69 ms. The Phase Review checks
  whether this cost grows faster than the source size.
- Depth 48 admits 15 nested loops or `if` statements. P6 measures
  whether a lower stack cost per level raises it.
- The absolute Luau ceiling of template parts is unknown.
