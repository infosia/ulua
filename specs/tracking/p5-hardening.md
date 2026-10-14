# P5 hardening

Status: open; P5.1 and P5.2 COMPLETE 2026-10-14. Decisions: D28, D29 (`plan.md` §8).

## Slices

| Slice | Family | Exit |
|---|---|---|
| P5.1 | Sanitizer build and gate | `sh tools/gate.sh sanitize` passes with zero reports |
| P5.2 | In-house fuzzing (D29): the compiler, the ABI call sequences, async request sequences | Time-boxed runs with zero invariant failures; each finding becomes a test |
| P5.3 | Fault injection: Luau allocator, host allocator, registry references | Each injection point gives its contract status; zero sanitizer reports, zero `internal` |
| P5.4 | Compiler memory bound | A measured bound per source byte, asserted as a ratio |
| P5.5 | Runtime review against pipeline.md §6.3 (GC and library internals) | No step whose time the bound does not cover |
| P5.6 | Linux and Windows (D28) | The owner's runs of `gate.sh full` pass on both |

## Carry-over into P5

- pipeline.md §6.3: a review of GC work and runtime internals outside
  the generated helpers (`tracking/p2-values.md`, P2.4).
- P4.3: no test injects an OS allocation failure or a `lua_ref`
  registry failure on the request path (`tracking/p4-async.md`).
- M1, M2 of P0.5 were measured on macOS arm64 only
  (`tracking/p05-measurements.md`).

## Measurements (P5.1)

- `sh tools/gate.sh sanitize` (macOS arm64, Apple clang, working
  symbolizer): `cargo-test` 345 s, `luau-witnesses` 55 s, docs witness
  3 s; zero sanitizer reports from the library code and from the
  pinned Luau sources. Codex's runs without a symbolizer and with
  `ASAN_SYMBOLIZER_PATH=/usr/bin/atos`: both pass, about 400 s.
- Seeded defects in `native/probes/sanitize.cpp` (not linked into
  either library), run in a child process: an 8-byte heap overflow
  gives ASan `heap-buffer-overflow` at `sanitize.cpp:12`;
  `INT_MAX + 1` gives UBSan `signed integer overflow` at
  `sanitize.cpp:16`. The tests are ignored in the normal build; the
  normal build has no sanitizer runtime.
- A first version of the probe test asserted the executable name in
  the report, which appears only without a symbolizer; it failed with
  a working symbolizer. It now asserts the seeded source line.
- `sh tools/gate.sh full` (normal build) passes: `cargo-test` 262 s,
  `luau-witnesses` 52 s.
- Not covered: LeakSanitizer (not on macOS arm64; leaks are P5.3), the
  Rust code (sanitizers for Rust need a nightly toolchain, D29), Linux
  and Windows (P5.6). Luau's small-block page allocator
  (`VM/src/lmem.cpp` of the pin) poisons only free blocks: an overflow
  inside a page, into the tail of a size class or a live neighbour
  block of 1,024 bytes or less, is invisible to ASan. The fuzzing of
  P5.2 and the fault injection of P5.3 do not depend on ASan for those
  objects.

## Exit checklist (P5.3)

- [ ] pipeline.md §10 implemented: the `fault-injection` feature, the
  allocator hook, the request table hook.
- [ ] The sweep over the set of §10.2 with the properties of §10.3,
  with its count of runs and its cost.
- [ ] A direct test shows that a seeded leak (a test-only allocation
  that is never freed) fails the live-bytes check.
- [ ] Each failure found becomes a test before its fix.
- [ ] Phase Review of the slice with no open CRITICAL or MAJOR finding.

## Measurements (P5.3)

Implementation record (Codex): the feature `fault-injection` (not in
the default or the `measurements` features) adds a hook to the budget
allocator and to the native request table; the test target `faults`
builds only with it. Sweep: 26 programs, 182 baseline runs, 1,239
allocation indices, each failed once in a fresh instance; 3,913 ABI
runs with the recovery calls; 0 failures; a seeded 31-byte leak fails
the live-bytes check; 0.37 s on the instrumented build. No library
defect and no contract question. Close gates: `gate.sh sanitize` 243 s
(fault sweep 2 s); `gate.sh full` after `cargo fmt` of
`crates/ulua/src/lib.rs` (Codex did not run it).

### P5.3 Phase Review round 1

Reviewer: fresh agent, diff `2ab2c56..e7ff5c3`. 0 CRITICAL, 0 MAJOR, 4 MINOR. The count covers every allocation that can fail: `fault_begin` runs before `lua_newstate`; a scratch counter saw 0 eligible allocations outside the counted runs. The count is low (about 35 to 47 per creation) because Luau serves small GC objects from size-class pages (`VM/src/lmem.cpp`); only a page or a large block reaches the allocator, in production as in the sweep. 1,079 of 1,239 indices fall in `ulua_instance_new`, 154 in calls, 2 in completions, 4 in settles; 63 fail inside script execution. Held: exactly the k-th allocation fails (a shrink is never refused); the completion hook matches abi.md §6.4.2; the usable-instance check uses the test's own fact; the sweep set matches §10.2; no build without the feature carries the hook.

| Finding | Severity | Resolution |
|---|---|---|
| `catch {` without a binding gets no handler marker (`a51`, `a52`) | MINOR | Mark every handler form |
| The protected-allocation program makes no counted allocation inside its `try` body, so no swept index fails there | MINOR | A large block inside the `try` body |
| The request-records check runs after `discard_records` and cannot fail (principle 9) | MINOR | Count the records from an independent fact before the discard |
| A replayed prefix is not checked to make its baseline count | MINOR | Assert the count |

## Exit checklist (P5.2)

- [x] pipeline.md §9 implemented: the `ulua_fuzz` crate, the four
  targets, the invariants, deterministic seeds, reduction, and the
  findings directory.
- [x] A direct test per invariant: a seeded violation (test-only) is
  reported as a failure, so a clean run is evidence.
- [x] The smoke runs in `sh tools/gate.sh sanitize`, with their cost.
- [x] 60 seconds per target with a fresh seed on the sanitizer build with zero open
  findings; each finding became a test before its fix.
- [x] Phase Review of the slice with no open CRITICAL or MAJOR finding.

## Measurements (P5.2)

Implementation record (a Claude general-purpose subagent, by the roles
exception of `rules-history.md`; Codex refused the handoff twice):

- `crates/ulua_fuzz` (binary `ulua-fuzz`), stable Rust, no new
  dependency: an in-crate PRNG (input `i` of seed `s` is `mix(s, i)`),
  a worker process per run under a supervisor (a signal, an abort, or a
  sanitizer report fails the running input), ddmin shrinking over
  bytes then tokens, findings under `target/fuzz/findings/`.
- Library finding (the `requests` target, an early trial, seed 5, 23
  inputs, shrunk to 28 bytes): on the sanitizer build,
  `VM/src/lvmexecute.cpp:779: runtime error: nan is outside the range
  of representable values of type 'int'`. A request completed with
  NaN; the async export returns it; `__ulua_resolve` reads the weak
  table `__ulua_promises[value]` with a number key, and the
  `LOP_GETTABLE` fast path of the pinned VM converts the key with
  `int(indexd)`, undefined for NaN and for numbers outside the `int`
  range. The normal build shows nothing. Tests first:
  `p52_findings::async_results_outside_the_int_range_resolve` and
  `host_completions_outside_the_int_range_resolve` (SIGABRT before the
  fix on the sanitizer build). Fix: the generated helpers read the
  table only when the value is a table; the value representation does
  not change.
- Two tool defects found and fixed in the runs (false failures): queued
  nested calls of a callback that never ran; a perturbed grant
  signature that stayed compatible.
- Smoke (seed `0x52f022d`, instrumented Debug): compiler 3,000 inputs
  (550 accepted, 6 witness samples) 10.3 s; run 3,000 (550) 10.5 s;
  abi 2,000 (1,760 compiling) 14.6 s; requests 1,500 20.3 s; 0
  failures; about 58 s in all.
- 10-minute runs (sanitizer build, seed `0x10ad`, the four at once):
  compiler 130,945 inputs (25,038 accepted, 368 witness samples); run
  127,317 (24,321, 359); abi 63,954; requests 34,496; 0 failures. One
  slow-compile report: the empty source (the fixed cost of the compiler
  thread); pipeline.md Rev 36 §9.3 measures inputs of at least 1 KiB.
- Contract questions decided in pipeline.md Rev 36: §3.7 rule ids
  (`C1` to `C19`); §9.2 the object pool (objects whose parent is freed,
  consumed ids; never a freed pointer); §9.3 the slow-compile metric.
- Gaps: the tsc CLI gives no syntax node, so TS2367 and TS2845 are
  exempt by code, not by the equality-node test of the corpus witness;
  the run target checks no output value; the abi model checks exact
  values for the fixed programs only.

### Gate split (principle 17)

The tests that assert time ratios (the `witnesses` test binary: 5
tests, 193 s inside gate full) build only with the feature `timing`
and run in `sh tools/gate.sh timing` (pipeline.md Rev 37 §3.10). Warm
build, macOS arm64: `gate.sh full` 116 s in total (`cargo-test` 55 s,
`luau-witnesses` 56 s; before the split about 330 s); `gate.sh timing`
200 s. Clippy runs with `--features ulua/timing`, so the timing tests
stay linted.

### P5.2 Phase Review round 1

Reviewer: fresh agent, diff `11f70a5..fb0469f`. 0 CRITICAL, 1 MAJOR, 6 MINOR, all in the tool or its tests; no library finding. Runs (instrumented build): seed 31249 for 60 s each, compiler 13,307 inputs (2,570 accepted), run 12,764, abi 5,646, requests 3,025, 0 failures; seed 40961, abi 356,243 and requests 195,069 inputs in one hour, 0 failures. Held: the library fix closes the class (every table key from a script value is guarded or a constant; 47 methods and operators with NaN, infinities, large and negative zero numbers give no UBSan report); 9 injected violations each give their key; reduction is deterministic; the mutations reach invalid UTF-8, surrogate escapes, nesting to 5,348, and sizes at the ceiling; the smoke is deterministic and under 60 s per target.

| Finding | Severity | Resolution |
|---|---|---|
| A `tsc` witness batch has no deadline (`witness.rs:130`) and the supervisor blocks on worker output: a 59,362-byte growth shape kept stock `tsc` at 100% CPU past 120 s; the compiler and run runs hung for 40 minutes and lost their findings | MAJOR | pipeline.md Rev 38 §9.3: 30-second deadlines per witness batch and per input; a witness timeout is a witness resource limit; findings are written when they occur |
| Caught generator panics in worker stderr key a later signal as `crash:panic`, so its replay does not reproduce | MINOR | Key from the signal first; generator panic messages stay out of the worker stderr |
| Reduction is 1-minimal only when its budget suffices, and the record does not say which | MINOR | §9.4: the record states it |
| The `tsc` witness omits `erasableSyntaxOnly` of `tsconfig.json` | MINOR | Use the options of `tsconfig.json` |
| A `tsc` stack overflow skips the whole batch | MINOR | Check the other inputs one at a time |
| The model expects `limit:memory` below 300,000 bytes, a measured value outside the contract | MINOR | Accept `ok` or `limit:memory` there |
| The tests of the fix lack infinities and negative zero | MINOR | Add them |

Fixes (the implementation subagent): a 30-second deadline per witness batch (the child is killed; a timeout or a `tsc` stack overflow checks each input again alone, and an input past the deadline alone is a witness limit record); a 30-second watchdog per input in the supervisor (key `hang`); findings written when they occur; `--seconds` always ends a run; crash keys from the sanitizer report and the signal before a panic message; a silent panic hook around the generators; `reduction.txt` states the reduction result; `tsc` runs with the options of `tsconfig.json`; the memory expectation accepts `ok` or `limit:memory`; tests for the infinities and negative zero. Stub tests show the deadlines (a `tsc` that sleeps, a worker that never ends). A 60-second compiler run with seed 40961 ends: 16,081 inputs, 3,100 accepted, 0 failures.

### P5.2 close

Review rounds: 1, with the MAJOR fixed and shown by stub tests. Close gates (warm build): `gate.sh full` 134 s, `gate.sh sanitize` 209 s (fuzz smoke 56 s). The timing gate did not run: no checker or emitter change since its last run (principle 17). Running tasks at the close: none.

## Exit checklist (P5.1)

- [x] pipeline.md §2.6 implemented: `ULUA_SANITIZE=1`, the flags,
  the separate target directory, the sanitizer runtime in test binaries
  and example hosts.
- [x] `tools/gate.sh sanitize` runs the tests, the Luau witnesses, the
  examples, and the docs witness on the sanitized build.
- [x] A direct test shows that the sanitized build reports a seeded
  defect (a test-only out-of-bounds write in a probe, not in either
  library), so a clean run is evidence.
- [x] Zero sanitizer reports over the whole sanitize gate; each report
  found on the way is fixed with a test.
- [x] The cost of the sanitize gate is measured and stated.
- [x] Phase Review of the slice with no open CRITICAL or MAJOR finding.

### P5.1 Phase Review round 1

Reviewer: fresh agent, diff `fb96851..97345a9`. 0 CRITICAL, 1 MAJOR, 5 MINOR. Held: 68 of 68 native compile commands carry the sanitizer flags and `-O1`, and their objects reference `___asan_*` (67 also `___ubsan_handle_*`; `lperf.o` has no checkable operation); 10 of 10 test executables and `libulua.dylib` link the ASan runtime; a seeded ASan and a seeded UBSan defect in the shim make `ulua-corpus` exit 134 and fail the Luau witnesses; the normal build has no sanitizer symbol; the probes cannot pass without the sanitizer; 365 hostile programs (nesting at the limits, the source ceiling, memory and step limits, request storms, 1M chained `then`) give zero reports.

| Finding | Severity | Resolution |
|---|---|---|
| The docs witness compares stdout only: a seeded use-after-free in the shim made the sanitized runner exit 134 after its full output, and the witness passed | MAJOR | `tools/witness/docs.mjs` fails on an unexpected status, a signal, or stderr text; a fake runner that prints the expected output, then a report, gives 10 failures of 10 |
| The ASan limit of Luau's small-block page allocator is not recorded | MINOR | Recorded under Measurements (P5.1) |
| `rustc-link-lib=asan`/`ubsan` matches gcc names only; clang on Linux names `libclang_rt.*` | MINOR | P5.6 (D28) measures the Linux link path |
| The gate header states no cost and has a stale shape list | MINOR | Header updated |
| `corpus-luau` and `stress-luau` ignore `CARGO_TARGET_DIR`, so concurrent gates delete each other's files | MINOR | They follow `CARGO_TARGET_DIR`; `luau-tools` stays shared (one uninstrumented analyzer) |
| User `ASAN_OPTIONS` and `UBSAN_OPTIONS` replace `halt_on_error=1`; the docs runner lacks `-fno-sanitize-recover` | MINOR | The gate appends its options last; the docs runner adds the flag |

### P5.1 close

Running tasks at the close: none.
