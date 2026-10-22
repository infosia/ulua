# P5 hardening

Status: COMPLETE 2026-10-16 (P5.1 to P5.6, Phase Review). Decisions: D28, D29 (`plan.md` §8).

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

## Compiler memory on x86_64 (pipeline.md Rev 56)

Status: COMPLETE on Windows x86_64 and macOS aarch64 (remeasured at
55ce513, section "macOS remeasurement" below), and on Linux x86_64
(section "Linux measured target" below).

Failure: `gate.sh timing` on Windows x86_64 at 7e08e23 failed
`compile_memory_bound`: derived base 85,504 > `COMPILE_MEMORY_BASE`
85,347. Largest small peak 68,403 B, seven distinct empty classes.
macOS aarch64 after P7 (`p7-stdlib.md`): derived base 85,180; the small
peak is not recorded (85,180 * 4 / 5 gives about 68,144 B).

Cause, measured on Windows (allocation backtraces at the peak):

| Source | Peak | Caller | Worker | Hash tables live | x86_64 extra |
|---|---:|---:|---:|---:|---:|
| `""` | 11,254 | 454 | 10,800 | 5 | 40 |
| `class A{}` | 34,986 | 454 | 34,532 | 16 | 128 |
| seven classes | 68,403 | 454 | 67,949 | 34 | 272 |

- A hashbrown 0.16.1 table has one control byte per bucket plus
  `Group::WIDTH`: 16 on x86_64 (SSE2), 8 on aarch64 (NEON). 34 live
  tables give 272 B more on x86_64.
- The seven-class peak without the 272 B is 68,131 B, against about
  68,144 B from the macOS derivation. The 13 B difference has no known
  origin.
- The only Windows-specific allocation is the UTF-16 thread name
  (28 B, worker). It is freed before the peak.
- The derived factor is 1,115 on Windows x86_64 (peak 58,409,165 B at
  65,536 B, nested-empty-blocks). The last macOS record is P5.4: peak
  58,408,895 B, factor 1,115.

Change: `COMPILE_MEMORY_BASE` 85,504 (the Windows x86_64 derivation);
`COMPILE_MEMORY_FACTOR` stays 1,115. `compile_memory_bound` on Windows
after the change: passed (107 s).

Consequence: a constant derived on one architecture does not bound
another. Each constant is the largest derivation over the measured
targets (macOS aarch64, Windows x86_64, and Linux x86_64 from D37).

## Compiler stack on Windows (pipeline.md Rev 72)

Status: COMPLETE on Windows x86_64 and macOS aarch64 (section "macOS
remeasurement" below), and on Linux x86_64 (section "Linux measured
target" below).

Failure (CRITICAL, invariant 2): on Windows x86_64 MSVC Debug at
9b18ef0, `ulua check` of a 265-byte source (41 nested method calls,
`class C{m(x:number):number{return x}}function f(c:C):void{` +
`c.m(` x41 + `1` + `)` x41 + `}`) stops the process: "thread
'ulua-compiler' has overflowed its stack". 40 calls pass. Release
passes at 41.

Found through `ulua_fuzz --smoke`: the witness recompile overflowed a
1 MiB main thread (fixed in 9b18ef0); a 40-deep array needed about
1.6 MiB in MSVC Debug, against the P1 record of 716,864 B.

The `measurements` feature did not compile (`ir::Module`,
`ir::Function` fields added after P1); no gate built it.

Measured on Windows with `s3_measure` (fixed), peak at the depth
ceiling 48, on a 32 MiB thread. P1 forms and every later nesting form
of the checker and the emitter (about 80 forms):

| Form | Count | Debug | Release |
|---|---:|---:|---:|
| Class-method calls as a statement (deepest) | 42 | 2,154,792 | 146,312 |
| Class-method calls in a return | 42 | 2,135,720 | 145,576 |
| Math calls in a return | 42 | 2,076,904 | 135,128 |
| Nested calls (P1) | 42 | 1,737,400 | 141,616 |
| For-of (largest Release) | 21 | 1,145,656 | 231,912 |

- Worst Debug slope: about 48,802 B per level; Release about 3,066 B.
- Thread bisection of the deepest form: Debug needs 2,162,689 B,
  Release 131,073 B.
- Run stack at `LUAI_MAXCCALLS`: protected C calls 158,000 B Debug,
  154,400 B Release; under `ULUA_MIN_RUN_STACK` 512 KiB.

Consequence: the half-stack rule needs at least 4,309,584 B on Windows
Debug. `COMPILER_STACK_BYTES` becomes 8 MiB on every target (owner
decision): the value is the largest derivation over the measured
targets, and the cost is address space of one thread per compile.
macOS is not measured with the later forms.

Change: `COMPILER_STACK_BYTES` 8 MiB. Tests in `s3_limits`:
`compiler_stack_abi` (42 nested method calls through `ulua_compile`;
43 give `limit:nesting-depth`) and `compiler_stack_half` (42 calls on
a thread of `COMPILER_STACK_BYTES / 2`). With the constant at 2 MiB
both stop with `STATUS_STACK_OVERFLOW` on Windows Debug; at 8 MiB both
pass. `ulua check` of the 41-call source exits 0. The `measurements`
feature builds again, and the clippy step of `gate.sh full` builds it
(exit 101 before the fix). `gate.sh full` on Windows: PASS.

## macOS remeasurement (55ce513)

macOS arm64, Apple clang, toolchain 1.95.0. Measurement only; no
production change.

Stack, `s3_measure stack --half` (the 13 P1 forms) and `s3_measure
source` on 30 later forms at their largest accepted depth. The later
sources come from a generator, later committed as
`tools/stack-forms.mjs` (32 forms; same peaks).

| Form | Count | Debug | Release |
|---|---:|---:|---:|
| Class-method calls as a statement (deepest) | 42 | 2,294,704 | 121,456 |
| Class-method calls in a return | 42 | 2,276,064 | 121,616 |
| `Math.abs` calls in a return | 42 | 2,212,896 | 120,944 |
| Nested calls (P1 `call`) | 42 | 1,852,624 | 120,448 |
| For-of (largest Release) | 21 | 1,224,448 | 228,624 |

- Debug peak 2,294,704 B against the Windows peak 2,154,792 B.
  `COMPILER_STACK_BYTES / 2` is 4,194,304 B. Every form gives
  `half_stack=true`.
- Release peak 228,624 B (Windows 231,912 B).
- Run time: `stack --half` Debug 34 s and Release 24 s with the
  build; the 30-form sweep 7 s.

Compiler memory, `compile_memory_bound` (feature `timing`, alone):
passed in 143 s. Derived base 83,617 B (small peak 66,893 B, seven
classes); derived factor 1,115 (peak 58,409,145 B at 65,536 B,
nested-empty-blocks). Both are at most their constants (85,504 and
1,115).

Defect found by the sweep (fixed, section "Depth fix (pipeline.md Rev
73)" below): a script input gives `internal`.
`ulua check` of 23 nested `try{}catch(e){}` with an empty innermost
block (364 B) reports `error[internal]: Emitter depth limit exceeded
after checked input`. `ulua_compile` gives status 6, code `internal`.
22 levels pass; 24 give `limit:nesting-depth`. `try{}finally{}` and a
nested catch body fail the same way at 23. Each try form with a
statement in the innermost block gives `limit:nesting-depth`.

Cause (read): `depth.rs` charges a try statement as two levels only
through the statements of its blocks. An empty innermost block has no
statement, so the AST check never charges its second level. The
emitter guard charges the try itself.

## Depth fix (pipeline.md Rev 73)

Status: COMPLETE 2026-10-22. Contract: `pipeline.md` §3 item 4 Rev 73.

Exit checklist:

- [x] No script input that `depth.rs` accepts reaches the emitter
  guard. Gate test: `depth_sweep_tests.rs` (the sweep and the direct
  try test).
- [x] The sweep is Red at 55ce513 and Green after the fix.
- [x] Every existing test passes unchanged
  (`condition_wrappers_reach_the_nesting_bound` included).
- [x] The deepest form stays under `COMPILER_STACK_BYTES / 2` in Debug
  on macOS.
- [x] Fresh review: 0 CRITICAL, 0 MAJOR.

Failures before the fix (the sweep at 55ce513): `internal` for
try/catch, try/finally, catch body, try/catch/finally, and finally
body at 23; empty template at 43; `true&&false` leaf at 42; `true&&`
and `false||` chains at 43.

Classes found:

| Class | Cause | Fix |
|---|---|---|
| Empty protected block | `depth.rs` charges the second try level through block statements only | `depth.rs` charges a try at itself (`depth + 1`) |
| Template string parts | the emitter lowers each string part as an expression; the AST has no child for an empty template | `depth.rs` charges a template at itself (`depth + 1`) |
| Double guard | `E::Logical` passes its guarded depth with the same node to `pure()`, which guards it again | `pure(expr, outer)`: the depth before the guard |

A first form charged every logical expression two levels in
`depth.rs`. It rejected `alternating-logical-tests` at depth 21, which
compiled before. The extra level was in the emitter, so the emitter
changed.

Accepted depth, 55ce513 and after:

| Form | Before | After |
|---|---:|---:|
| `true&&` / `false\|\|` chain | 42 (43 gives `internal`) | 43 |
| `true&&false` leaf in blocks | 41 (42 gives `internal`) | 42 |
| Empty try forms | 22 (23 gives `internal`) | 22 |
| Empty template in blocks | 42 | 42 |
| Function declarations around an empty try (review) | 22 | 21 |
| Empty template in parentheses (review) | 43 | 42 |
| Empty template in parentheses in blocks (review) | 42 | 41 |

The losses follow the contract: a form charges the levels of its full
form.

Stack (Debug, macOS arm64): `true&&` x43 peak 1,573,120 B; the
deepest form (42 method calls) 2,294,704 B, unchanged.

Gates: `cargo test -p ulua_compiler` 458 passed; `gate.sh quick` PASS
(842 passed, 352 s, before the review fixes); sweep 81 shapes at depths
1 to 64 (5,184 inputs), 1.56 s, on a `COMPILER_STACK_BYTES` thread.
`gate.sh full` after the review fixes (macOS arm64): PASS, 356 s.

Review round 1 (fresh reviewer): 0 CRITICAL, 0 MAJOR, 4 MINOR.

| Finding | Severity | Resolution |
|---|---|---|
| §3 item 4 said "the checker charges"; the bound is `depth.rs` | MINOR | contract text names `depth.rs` |
| Some empty shapes lose one level (table above) | MINOR | accepted: the contract rule |
| The direct test runs on the default test thread | MINOR | runs on a `COMPILER_STACK_BYTES` thread |
| The sweep omits error field, JSON, length, push, for init, string for-of, reduce, filter | MINOR | rows added |

Reviewer sweep: about 3,600 shapes in 18 statement and 8 expression
contexts at 12 depth limits, 0 `internal` (74 at HEAD).

## Linux measured target (D37)

Status: measured at ebf2fed. Linux x86_64, gcc 15.2.0, kernel 7.0.
Owner decision D37: Linux x86_64 is a measured target (pipeline.md
Rev 74, benchmarks.md Rev 6).

Failure before the fix: `native/probes/s3.cpp` had a stack watermark
for macOS and Windows only. On Linux the probe returned 0.
`node tools/stack-forms.mjs` printed peak 0 for all 32 forms in Debug
and Release, and `stack-forms: PASS`. The accepted depths were
correct; no form gave `internal`.

Change: a Linux watermark from `pthread_getattr_np` on a spawned
thread. On the main thread the probe stays unstarted: glibc derives
the main-thread bounds from the rlimit (44,888,887,422,976 B under
`ulimit -s unlimited`, review round 1). `s3_measure` exits 2 on a zero
stack peak; `stack-forms.mjs` fails on a zero peak. Every
`s3_measure` stack measurement runs on a spawned thread.
Red: `s3_measure stack --half` with the zero check and without the
Linux watermark exits 2.

Compiler stack, `tools/stack-forms.mjs` (32 forms, 7 s):

| Form | Depth | Debug | Release |
|---|---:|---:|---:|
| Class-method calls as a statement (deepest) | 42 | 2,179,832 | 119,688 |
| Class-method calls in a return | 42 | 2,161,976 | 119,848 |
| `Math.abs` calls in a return | 42 | 2,102,840 | 118,328 |
| Nested calls as a statement | 43 | 1,812,232 | 120,568 |
| For-of (largest Release) | 21 | 1,181,864 | 229,136 |

- Debug peak 2,179,832 B: between Windows (2,154,792 B) and macOS
  (2,294,704 B). Half of `COMPILER_STACK_BYTES` is 4,194,304 B.
  `COMPILER_STACK_BYTES` stays 8 MiB.
- `s3_measure stack --half` (13 P1 forms): every form passes with
  `half_stack=true`. Largest Debug peak: `call` at 42, 1,760,696 B.

Run stack, `s3_measure native` at `LUAI_MAXCCALLS`: protected C calls
144,200 B Debug, 129,544 B Release; direct C calls 64,600 B Debug.
Windows x86_64 Debug has the largest value, 158,000 B. All values are
below `ULUA_MIN_RUN_STACK` (512 KiB).

Compiler memory, `compile_memory_bound` (Debug, 115 s): factor peak
58,409,249 B for `nested-empty-blocks` at 65,536 B, derived factor
1,115; small peak 67,165 B for seven empty classes, derived base
83,957. Native largest peak 51,616,892 B. `COMPILE_MEMORY_FACTOR`
1,115 and `COMPILE_MEMORY_BASE` 85,504 do not change.

Performance, `p63_performance`, three runs (each about 4 s):

| Workload | Run 1 | Run 2 | Run 3 | Linux bound | Old bound |
|---|---:|---:|---:|---:|---:|
| fib-recursive | 1.827 | 1.663 | 1.742 | 2.29 | 2.18 |
| fib-loop | 1.437 | 1.282 | 1.438 | 1.80 | 2.85 |
| mandelbrot | 1.745 | 1.758 | 1.760 | 2.20 | 2.06 |
| primes | 1.113 | 1.110 | 1.110 | 1.40 | 3.62 |
| sort | 3.060 | 3.074 | 3.051 | 3.85 | 4.54 |
| tree | 1.987 | 1.942 | 1.945 | 2.49 | 2.88 |
| queen | 3.179 | 3.040 | 3.180 | 3.98 | 5.20 |
| particles | 1.855 | 1.852 | 1.858 | 2.33 | 4.42 |
| callbacks | 1.392 | 1.395 | 1.393 | 1.75 | 3.98 |
| collect | 1.802 | 1.814 | 1.801 | 2.27 | 2.61 |

The test file holds the nine-digit values. The bounds of
fib-recursive and mandelbrot rise to the Linux derivation. The other
bounds do not change.

Not measured on Linux: the heap modes of `s3_measure`. They need the
macOS heap sampler and exit 2 on another target. No contract value
derives from them per target.

Review round 1 (fresh reviewer): 0 CRITICAL, 1 MAJOR, 4 MINOR. Held:
the Linux watermark over-reports by 4,688 B on a spawned thread (a
scratch program with a known `alloca`); every stack peak of
`s3_measure` passes the zero check; the ten bounds match the rule.

| Finding | Severity | Resolution |
|---|---|---|
| This note said Linux is not measured and held no Linux numbers | MAJOR | This section; status lines updated |
| pipeline.md §7 table had no Linux peak; the `ULUA_MIN_RUN_STACK` row was not the largest value | MINOR | Rows updated; `include/ulua.h` names 158,000 B |
| D37 is broader than the code: heap modes print 0 on Linux | MINOR | Rejected: `s3_measure` exits 2 for a heap mode on a target other than macOS |
| The main-thread bounds from glibc are the rlimit: terabytes under `ulimit -s unlimited` | MINOR | The probe stays unstarted on the main thread; the end returns 0 |
| benchmarks.md line wrap; the SAFETY comment of `stack_peak()` | MINOR | Fixed |

Review round 2 (fresh reviewer): 0 CRITICAL, 0 MAJOR, 3 MINOR. A
status line of the Rev 72 section omitted Linux; the SAFETY comment
of `stack_peak()` claimed that an unstarted probe gives 0, which is
false on macOS; the main-thread comment and this note did not state
the exclusion. All three fixed.

`gate.sh full` on Linux with the slice: PASS (`cargo-test` 115 s,
`luau-witnesses` 56 s).

## Header names (pipeline.md Rev 76)

Status: COMPLETE on Windows x86_64.

Failure: `gate.sh full` on Windows at a65f21c failed `case_memory`
(`reservation_tracks_only_host_temporary_and_releases_before_lua`).
The test compiles `case_budget.cpp` with `/I native/shim`. cl.exe
stdout: `cstdlib(24): error C2039: 'fabs'` (also `fabsf`, `fabsl`):
`native/shim/math.h` shadowed the standard `<math.h>`. The failure
message was empty: the test printed only stderr, and cl.exe writes its
diagnostics to stdout.

Class: 7e08e23 removed `native/shim` from the include path of
`ulua_bench` for the same cause, and kept the header name, so the
class came back in a new test.

Change: `native/shim/math.h` is `native/shim/ulua_math.h`. pipeline.md
§2 item 7: no header of the repository has the name of a standard C
or C++ header. `tools/header-names.mjs` checks it in `gate.sh quick`
and `full` (exit 1 at a65f21c, 0 after the rename). The test prints
the compiler stdout and stderr, and passes `/utf-8` (C4819 on the
UTF-8 text of the Luau headers under code page 932 after the rename).
`case_memory`: 2 passed, 1 failed before; 3 passed after.

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

## Measurements (P5.6)

- Windows (owner's run, MSVC, commit 4c1bb2c): `gate.sh full` passes;
  `cargo test` 353 passed, 2 ignored (sanitizer probes), 97 s; luau
  witnesses 56 s. Fixes: `layout.c` builds as C11 under MSVC (no
  `_Generic` before `/std:c11`); `-fno-math-errno` only where the
  compiler accepts it; witness paths through `fileURLToPath`; `tsc`
  file names normalized before the prelude filter (TS2304 on 199
  entries before); `luau-analyze` in batches of 64 files (error 206
  before); CRLF output compared exactly.
- Windows after P6 and P7 (owner's run, MSVC): at 7e08e23
  `gate.sh full` passes. Fixes: `ulua_bench` had `native/shim` on the
  include path, so `<cstdlib>` read `native/shim/math.h`
  (`/showIncludes`); the compiler memory probe passed gcc arguments to
  `cl.exe` and wrote an object into `crates/ulua`; `docs-runner.sh`
  wrote `docs-run.exe.exe` (`cygpath -w` adds `.exe` to an existing
  executable), so the docs witness ran a stale runner. At dcaae63
  `gate.sh timing` passes after pipeline.md Rev 56 and benchmarks.md
  Rev 5 (section "Compiler memory on x86_64" above;
  `p6-performance.md`). At 15b9cd9 `gate.sh full` passes
  (`p8-cli.md`). `gate.sh sanitize` is not run on Windows.
- Windows `gate.sh sanitize` at 56ca47e: `cargo-test` fails to link.
  `build/sanitize.rs` passes `-fsanitize=address,undefined` to
  `link.exe` (LNK4044, ignored) and links `asan` (LNK1181, no
  `asan.lib`). Owner decision D36: Windows is outside the sanitizer
  build; `gate.sh sanitize` exits 2 on Windows.
- Windows, the steps of `gate.sh sanitize` that do not need the
  sanitizer build, on the normal build at 56ca47e: the fault sweep
  passes (5 tests, 38 s). `ulua_fuzz --smoke` exits 1: target `run`
  gives 2 failures with the key `crash:exit:-1073741571`
  (0xC00000FD, `STATUS_STACK_OVERFLOW`); targets `abi` and `requests`
  give 0. Cause: the witness recompile on a 1 MiB main thread. Fixed
  in 9b18ef0 (section "Compiler stack on Windows" above).
- macOS arm64 at 158c947 (`COMPILER_STACK_BYTES` 8 MiB): `gate.sh full`
  passes; `cargo-test` 308 s with the rebuild, `cargo-clippy` 44 s with
  the `measurements` feature. `s3_measure` with the later forms is not
  run on macOS.
- macOS arm64 after 4c1bb2c: `gate.sh full` passes in 166 s warm
  (`cargo-test` 98 s, `luau-witnesses` 63 s); `gate.sh timing`
  passes. A first run after the pull took 442 s with a cold build.
- Linux x86_64 (owner's machine, gcc 15.2.0, kernel 7.0, commit
  c693b77): `gate.sh full` passes with no source change. `cargo-test`
  100 s, `luau-witnesses` 85 s, `luau-tools` 38 s, `cargo-clippy`
  38 s. The documented Linux link tail `-lstdc++ -lpthread -ldl -lm`
  is the tail that the host tests and the docs runner use. gcc gives
  `-Wbidi-chars` and `-Wnonnull` warnings in the pinned Luau sources
  (`Ast/src/Confusables.cpp`, `CodeGen/src/CodeGenLower.h`); the build
  does not fail on them.
- Linux `gate.sh sanitize`, gcc 15.2.0, before the fix: `cargo-test`
  fails `sanitize::heap_write_reports_asan`. gcc's UBSan object-size
  check reports the seeded write at `sanitize.cpp:12` ("store to
  address ... with insufficient space") and exits 1 before ASan
  reports. With that test skipped, the other steps pass with zero
  sanitizer reports.
- Fix: the probe's pointer object is volatile, not its pointee. Two
  separate volatile loads hide the allocation from object-size and
  keep the store live. Reviewer measurement: g++ 15.2.0 and clang++
  21.1.8 give ASan `heap-buffer-overflow` at `sanitize.cpp:12` at -O1,
  -O2, and -O3; the overflow probe reports at `sanitize.cpp:16`.
- Linux `gate.sh sanitize` after the fix: PASS. `cargo-test` 166 s,
  `luau-witnesses` 108 s, `fuzz-smoke` 125 s, `fault-sweep` 57 s. The
  gcc sanitizer link path (P5.1 review, MINOR) works.
- macOS arm64 `gate.sh sanitize` after the probe change (00867ca):
  PASS. `cargo-test` 245 s, `luau-witnesses` 164 s, `fuzz-smoke` 67 s,
  `fault-sweep` 105 s; total 585 s.
- Linux `gate.sh timing` at e7a23a3: PASS. 9 passed, 2 ignored; the
  `witnesses` test binary 330 s, the step 367 s.
- Gap: a clang-built probe on Linux hangs in llvm-symbolizer with
  default symbolization (old and new probe); the Linux gate uses gcc.

## P5 Phase Review

Cross-slice review of fb96851..HEAD: 0 CRITICAL, 0 MAJOR, 7 MINOR.

| Finding | Severity | Resolution |
|---|---|---|
| `COMPILE_MEMORY_BASE` 85,022 is below its derivation after P5.5: small peak 68,277, derived 85,347; the test never asserts derived <= constant | MINOR | `COMPILE_MEMORY_BASE` 85,347; the test asserts each derived value <= its constant |
| The fault sweep set has no program with a spill table (13 entries, none with `__ulua_spill`) | MINOR | two programs join the sweep: 11 captured bindings (38 allocation indices) and 177 locals (39); the sweep checks `__ulua_spill` in each; 1,352 indices, 0.14 s |
| The `gate.sh` header states sanitize about 4 minutes; measured 585 s (macOS), about 456 s (Linux) | MINOR | header: 8 to 10 minutes |
| No gate lints the fault-injection code | MINOR | gate full clippy adds `ulua/fault-injection` |
| On Windows a fuzz crash is keyed by the panic message, not the signal | MINOR | closed 2026-10-22: Windows has no signal; `crash_key` uses the panic message, then the exit code. Measured at 56ca47e: a stack overflow gives `crash:exit:-1073741571` |
| pipeline.md §7 records prelude 1,222 B; it is 2,086 B | MINOR | §7 updated |
| `sanitize.rs`, `p52_findings.rs`, `capture_tests.rs` state no measured cost | MINOR | headers state the cost |

The reviewer's fuzz run (§9.5, sanitizer build, seed 1016531, 60 s
per target): 0 failures.

P5 close (macOS arm64): `gate.sh full` 165 s, `gate.sh timing` 380 s,
`gate.sh sanitize` 303 s (`fault-sweep` 37 s), hygiene passes. Open:
the Windows fuzz crash key (MINOR, tool only) (closed 2026-10-22, table above). Running tasks at the
close: none. End to end through the C ABI: limit:steps,
limit:memory, and the source ceiling stop as the contract states.

## Exit checklist (P5.5)

- [x] A fresh reviewer audits each run-time operation that does work
  outside the interrupt count: Luau library calls that the emitted
  code and the helpers make, VM operations on strings and tables, GC
  steps, error unwinding, and the async job queue.
- [x] For each operation: the work of one step is constant, or in
  proportion to its allocation charge, or a finding with a measured
  shape (time of one step, at the default limits, Release).
- [x] Each finding is fixed, contract first, with a test; or
  pipeline.md §6.3 names the bound that covers it, with the measured
  worst case.
- [x] The P5.4 Phase Review runs in the same review.

## P5.4 and P5.5 Phase Review round 1

| Finding | Severity | Resolution |
|---|---|---|
| The base came from the empty source (8,897 B). Small sources exceed the bound: `class A{}` (9 B) peak 34,363 > 20,671; `;` 18,344 > 12,183 | MAJOR | pipeline.md Rev 41: a small-source set gives the base |
| Nested empty blocks give 891.2 B per source byte at 64 KiB, above the recorded maximum 849 | MINOR | the shape joins the test |
| Native `luau_compile` was measured for the 5 largest outputs; the peak per output byte ranges 2.7 to 18.7 by shape | MINOR | each shape at 64 KiB is measured |
| The test measured `compile_with_grants`, not `ulua_compile`; the ABI copies the bytecode while the Luau output is live | MINOR | the test calls `ulua_compile` |
| A straight-line function body runs as one step: 64 KiB of `x=x+...+x;` is 63,807 instructions, one step of 112 to 122 us; `while(true){f();}` runs 38.2 s before `limit:steps` | MAJOR | pipeline.md Rev 42 §5.19: interrupt points |
| One GC step traverses a whole table and clears weak tables in the atomic phase: 0.078 ms at 5 MB live, 1.37 ms at 34 MB; 20M steps take 0.88 to 1.04 s at each heap size | MINOR | §6.3 names it; the collector work follows the charged allocation |

Candidates found covered (reviewer, with source lines of the pin):
string ordering (rejected, C5); string equality (interned); string
creation and hashing (in proportion to allocation); substrings (pieces
of at most 256 bytes); searches (KMP Luau loops); `repeat`, pads, trims;
array methods (Luau loops); `xs.length = 0` (a Luau loop); `table.insert`
(append form only); `table.clear` (once per settle or run); `#t` (dense
arrays); `__ulua_concat`, `__ulua_format`; the async drain and the
rejection scan; error unwinding (each frame is entered by a call); the
allocator charge.

Interrupt point forms (reviewer, pinned VM, O1): `for _ = 1, 1 do end`
fires 1 interrupt, 5 instructions, 5.7 ns; a no-op call fires 2, 6.7
ns; `repeat until true` and `while false do end` are folded and fire
none. With the loop form every 16 statements, the 64 KiB shape has a
longest gap of 2.9 us (32: 5.3 us; 64: 9.6 us) and the same call time
(118.6 us).

Round 1 fixes of P5.4: 277 shapes and 166 small sources through
`ulua_compile`, Debug and Release equal. Largest ratio 891.25
(`nested-empty-blocks`, 64 KiB); largest small peak 68,017 B (seven
empty classes). `COMPILE_MEMORY_FACTOR` 1,115, `COMPILE_MEMORY_BASE`
85,022. Native `luau_compile`: largest peak 44,158,440 B
(`dense-template`); largest 21.49 B per output byte
(`dense-nested-functions`). Test cost: 80.7 s Debug, 22.0 s Release
(a first version cost 135 s; a second Rust compile per shape for the
native input came out). The measurement hook is under feature
`timing` only.

## Measurements (P5.5)

Interrupt points (pipeline.md §5.19), measured by the implementation
under `lua_singlestep` with the production options (O1), 64 KiB:

- Largest instruction gap over 45 shapes: 34 (before: 74,848 for
  `s=s+'';`, 63,971 for the 42-term addition, 65,478 for a long
  `finally` body). `INTERRUPT_INSTRUCTIONS` 44.
- Ratio to `while (true) { i++ }` at the default `max_steps`
  (Release): largest 4.58 (boolean template); addition 2.12 (before:
  5,633). The addition call takes 113.9 us (114.3 us before the
  change).
- First implementation: an inline `for _ = 1, 1 do end` and one node
  per pure statement. It failed the gate: `luau-analyze` gave "Code is
  too complex to typecheck" on P25-logical-wrapped-tests at 16 KiB,
  and the analyzer time of S4-module-bindings rose to 1.47 s (0.13 s
  with the call form). The final form: `__ulua_interrupt()` and 3
  nodes per pure statement.
- The interrupt helper is one more captured upvalue and one more
  module temporary; the upvalue bound tests count it (192 passes, 193
  fails). The S4 bytecode prefix of unspilled fixtures grew from 999
  to 1,039 bytes.
- Test costs (Debug): gaps 13.4 s, strict 61 inputs 2.3 s (runtime
  target); ratios 45 shapes 16.5 s (witnesses, `timing`). Gate timing
  takes 373 s.

## P5.5 Phase Review round 2 (interrupt points)

| Finding | Severity | Resolution |
|---|---|---|
| `INTERRUPT_INSTRUCTIONS` 44 was fitted: a 32-argument call (33 moves and the call) after 15 counted statements gives a gap of 49; `new K(32 args)` and a 32-argument arrow call also give 49 | MAJOR | pipeline.md Rev 43: a derived bound from `STATEMENT_INSTRUCTIONS` |
| A forward loop exit escapes the count: `while(true){7 x++; if(b){break;} 3 x++}` then 20 `x++` gives 31 against 18 straight; a condition exit gives 24 | MAJOR | Rev 43: the count after a loop joins each exit |
| The async prelude has a gap of 47 at `__ulua_async_register`; the probe cannot run async modules, so no async shape is measured | MAJOR | Rev 43: the prelude rule and async shapes |
| The shapes miss the three classes above; the expected value is a fitted maximum | MINOR | Rev 43: two derived assertions |
| Cost changes recorded in part: two interrupts per point use steps; spill headers store one field per statement with an interrupt point after each 16; output density rose (field-inc 33.0 to 36.8 bytes per byte) | MINOR | the round 2 fix records the `OUTPUT_FACTOR` margin and the spill entry cost |

Round 2 fixes: `STATEMENT_INSTRUCTIONS` 36, `INTERRUPT_INSTRUCTIONS`
579 (derived). Largest gap 49, largest statement 36 over 195 cases.
Loop exits: break 24 to 18, continue 22 to 19, condition exit 24 to
18, nested loops 30 to 18, for-of 24 to 20. Async prelude and resume
paths 47 to 23; a numeric-arrow prelude run 37 to 18. A hand-written
40-argument Luau call (42 instructions) shows the statement assertion
Red; no accepted program at 0e3146e exceeds 36. `STORE_WEIGHT` 8
(table above in pipeline.md §5.19); the earlier rule (an interrupt
point after each store) cost 614 steps per 128-field spill entry.
Largest ratio 4.77 over 64 shapes. Largest growth output factor
66.56 (`spillD`) against 80.

## P5.5 Phase Review round 3

| Finding | Severity | Resolution |
|---|---|---|
| A closure over N captured locals of its own frame does work in proportion to N squared in one `NEWCLOSURE` (`luaF_findupval`), with no allocation after the first iteration: ratio 120 at N 130 (0.80 s against 6.7 ms) | MAJOR | pipeline.md Rev 44 §5.14.5a `CAPTURE_LOCALS` |
| The ratio test asserts 6; §5.19 said 8 | MINOR | Rev 44: 6 |
| Fixed helper bodies run up to 17 statements without an interrupt (`__ulua_sf`, gap 44) | MINOR | Rev 44 names them; the gap is measured |
| The derivation 16 × 36 + 3 omits uncounted lines (`function(` arguments, `nil`, `else`); a corpus entry ran 18 statement lines, gap 20 | MINOR | Rev 44 states why the bound holds |
| The test cost of the gap test: header 8.69 s, this note 13.4 s | MINOR | this note: the header value is the test alone; 13.4 s was the runtime target |

The reviewer found no program above 579 instructions or a statement
above 36 (130 corpus entries, 32-argument calls of each kind, 1,000
element and field literals, 500 template substitutions, nested loop
exits). Round 3 fixes: `CAPTURE_LOCALS` 10; the 11th and later captured
bindings of a function spill (parameters, defaults, per-iteration
bindings, catch bindings included). Ratios after the fix: N 10 4.92,
11 5.29, 130 1.98, 190 1.97 (Release); at 8878873 N 130 gave 117.6
and N 190 gave `limit:memory`. The capture ratio test is Red at
8878873.

The `limit:memory` of the 180-binding closure loop at 8878873 is GC
pacing: at 16 MiB it stops after 96,117 steps with 16,777,124 bytes
in use, at 32 MiB after 195,142 steps; the collector is in the
remark phase and completes no cycle. A GC goal of 1000 instead of
200 (a measurement only) keeps the peak at 548,324 bytes over 1,549
cycles. A full collection leaves 379,252 bytes; no leak. After the
fix the loop peaks at 515,488 bytes at the default settings. Gap:
GC pacing against `max_memory_bytes` (§6.5) for P6.

P5.5 close: `gate.sh full` 187 s, `gate.sh timing` 378 s, hygiene
passes. Running tasks at the close: none.

## Exit checklist (P5.4)

- [x] pipeline.md §6.4 implemented: `COMPILE_MEMORY_FACTOR` and
  `COMPILE_MEMORY_BASE` from the measured maximum with a margin of one
  quarter, and the test over every growth shape at 16 and 64 KiB.
- [x] The native `luau_compile` memory of the largest outputs is
  measured and recorded.
- [x] The test cost is stated; the test runs in `gate.sh full` if it
  costs at most 10 s, else in `gate.sh timing` (principle 17).
- [x] Phase Review of the slice with no open CRITICAL or MAJOR finding.

## Measurements (P5.4)

Implementation record (Codex): every growth shape compiled at 16 and
64 KiB under a counting global allocator (the peak live Rust heap of
one compile; requested sizes, without allocator overhead, stack, or
RSS); Debug and Release give the same counts; no shape above 2,000
bytes per source byte. Constants (the measured maximum and base, each
times 1.25, rounded up): `COMPILE_MEMORY_FACTOR` 1,061 bytes per
source byte, `COMPILE_MEMORY_BASE` 11,122 bytes. Native `luau_compile`
of the 5 largest outputs (64 KiB inputs, 4.1 to 4.3 MB of Luau): peak
42.2 to 42.8 MB, each compiles. The test costs 60.5 s Debug (10.4 s
Release), so it runs in `gate.sh timing` (now 259 s). Close gates:
`gate.sh full` 185 s, `gate.sh timing` 259 s. The Phase Review of P5.4
runs with the review of P5.5.

## Exit checklist (P5.3)

- [x] pipeline.md §10 implemented: the `fault-injection` feature, the
  allocator hook, the request table hook.
- [x] The sweep over the set of §10.2 with the properties of §10.3,
  with its count of runs and its cost.
- [x] A direct test shows that a seeded leak (a test-only allocation
  that is never freed) fails the live-bytes check.
- [x] Each failure found becomes a test before its fix.
- [x] Phase Review of the slice with no open CRITICAL or MAJOR finding.

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

Fixes (Codex): every handler form carries a marker; the protected-allocation program allocates a large block inside its `try` body, and 18 swept indices fail after the `try` began with no `catch` or `finally` marker; the records check counts open requests from the test's own model before the discard; each replayed prefix makes its baseline count. Sweep: 26 programs, 1,257 indices, 3,967 ABI runs, 0.371 s.

### P5.3 close

Review rounds: 1. Close gates: `gate.sh full` 170 s, `gate.sh sanitize` 246 s. Running tasks at the close: none.

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
