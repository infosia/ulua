# P6 performance

Status: P6.S7 OPEN 2026-10-18 (proven index). P6.S6 COMPLETE 2026-10-17 (native remainder, after the roblox-ts comparison). P6 COMPLETE 2026-10-16 (reload left P6, D31); P6.1, P6.S1, P6.2, P6.S2, P6.S3, P6.4, P6.S4, P6.S5, P6.3 COMPLETE 2026-10-16. Contract: `blocks/benchmarks.md` Rev 1 (Luau and Ulua only, owner decision 2026-10-16).

## Slices

| Slice | Family | Exit |
|---|---|---|
| P6.1 | Luau and Ulua suite (`benchmarks.md` §1 to §4) | Both subjects give the same checksum on all ten workloads; the noise gate passes or withholds; `results.json` and the README table are written by one full run |
| P6.2 | Boundary price (§5) | ns per call for both boundaries, each layer's share, a named shrink candidate or "the floor"; no production change |
| P6.3 | Performance gate (§6) | A ratio test in `gate.sh timing` with a bound from the P6.1 record; Red with a seeded slowdown |
| P6.4 | GC pacing against `max_memory_bytes` (P5 carry-over) | `pipeline.md` §6.5 settings; the 18-frame spill loop ends with `limit:steps` at 4 and 16 MiB; the §5.19 ratio tests pass |

Shrink slices follow the `ulua / luau` column of P6.1, each with a
stated problem and evidence (principle 13). They run before P6.3, so
the gate bound comes from the record after the shrinks.

| Slice | Family | Exit |
|---|---|---|
| P6.S1 | Builtin aliases and the index check (`pipeline.md` §5.20) | Each builtin alias compiles to `FASTCALL`; the trap set of the array helpers is unchanged (tests at each trap edge); the §5.19 tests pass; a new full record |
| P6.S2 | Loop form (`pipeline.md` §5.21, C9.2) | The corpus and goldens are unchanged; C9 entries (`a07`, `a09`, `a17`, `a65`) pass; the §5.19 tests pass; a new full record |
| P6.S4 | Loop-head interrupt point, inline index check, native equality (`pipeline.md` §5.19, §5.20, §5.23) | The corpus and goldens are unchanged; the C6 trap entries pass; tests at each index trap edge equal the helper; the §5.19 tests pass; a new full record |
| P6.S5 | Conditions with `&&` and `||`; counted loops (`pipeline.md` §5.22, §5.24) | The corpus and goldens are unchanged; `a67` pins the counted-loop edges; the §5.19 and analyzer tests pass; a new full record |
| P6.S3 | Expression form (`pipeline.md` §5.22) | The corpus and goldens are unchanged; C12 entries (`a18`, `a31`, `a50`, `a66`) pass; the §5.19 tests and the analyzer stress gate pass; a new full record |
| P6.S6 | Native remainder (`pipeline.md` §5.25, C4.3) | The corpus and goldens are unchanged; `a68` pins the fact edges; a unit test for each fact rule and each no-fact form checks the emitted form; the shadow check gives no stop over the corpus, the workloads, and the differential generator; the growth test includes a `FACT_ROUNDS` shape; the §5.19, analyzer, and P6.3 tests pass; a new full record in which primes falls and no workload rises outside the noise gate |
| P6.S7 | Proven index (`pipeline.md` §5.26, C6) | The corpus and goldens are unchanged; `a87` and `t17` pin the proven-index edges; a unit test for each index form (read, write, compound write, literal, counter, no-fact index) checks the emitted test; the shadow check gives no stop over the corpus and the workloads; the trap-edge tests of the index check pass; the §5.19, analyzer, and P6.3 tests pass; `bench --only` before and after for queen, particles, and sort in one session; a new full record in which no workload rises outside the noise gate |

## Carry-over into P6

- P5.5: a loop that makes garbage at the 180-binding capture shape
  reached `limit:memory` with a live heap near 400 KB; the collector
  completed no cycle (`tracking/p5-hardening.md`, Phase Review round 3).
- P5 Phase Review: on Windows a fuzz crash is keyed by the panic
  message, not the signal (tool only).

## Measurements (P6.1)

MacBook Air, Apple M2, AC power, Release; `max_steps` 10^10,
`max_memory_bytes` 1 GiB. Checksums equal in both subjects; no cell
withheld; largest IQR 1.69% of the median.

| Workload | Checksum | luau ms (1.00×) | ulua ms | ulua / luau |
|---|---:|---:|---:|---:|
| fib-recursive | 1346269 | 61.0 | 186.9 | 3.06 |
| fib-loop | 3449993952 | 113.8 | 1825.7 | 16.04 |
| mandelbrot | 11189274 | 504.0 | 981.9 | 1.95 |
| primes | 17984 | 80.8 | 704.4 | 8.72 |
| sort | 3135319498 | 52.3 | 406.8 | 7.79 |
| tree | 393204 | 35.7 | 108.7 | 3.05 |
| queen | 2680 | 327.0 | 2035.1 | 6.22 |
| particles | 77504288 | 84.1 | 611.7 | 7.27 |
| callbacks (Ulua methods, Luau loops) | 3403969141 | 102.2 | 458.2 | 4.48 |
| collect | 1011185172 | 127.7 | 571.1 | 4.47 |

Parameters: `crates/ulua_bench/README.md`. The Luau workloads use
idiomatic Luau (numeric `for`, the native `%`). Test cost: 6 unit tests
0.23 s (10 compiles and `tsc` 0.20 s). `gate.sh full` passes, 163 s.

## P6.1 Phase Review round 1

| Finding | Severity | Resolution |
|---|---|---|
| fib-loop: Ulua evaluates `j < i % 40` on each inner iteration (20.5M), the Luau numeric `for` once per outer iteration (1M); Luau with the same work 348 ms against 112 ms; 16.0 published, 5.1 for the same work | MAJOR | `benchmarks.md` Rev 2: a pair does the same operations; the bound is hoisted |
| queen, particles, sort: the Luau baselines use `math.abs` and `math.floor`, which Ulua lacks; queen with the Ulua operations 421 ms against 326 ms | MINOR | Rev 2: no library function in the Luau workload |
| `results.json` names the parent commit with `worktree_dirty: true` | MINOR | Rev 2: a full run from a changed tree writes no record |
| The host identity uses `DefaultHasher`, which is not stable across Rust releases *(docs)* | MINOR | a fixed hash |
| `subjects::measure` (warm-up loop, sample count) has no direct test | MINOR | a direct test |

Decomposition of `ulua / luau` (reviewer, measurement only, layers
removed in order, shares of the difference):

| Layer | fib-loop | fib-recursive | primes | sort | queen |
|---|---:|---:|---:|---:|---:|
| Different work in the pair | 14.1% | 0 | 0 | 2.9% | 8.8% |
| Interrupt callback and charges | 13.0% | 15.0% | 8.7% | 6.4% | 11.6% |
| `__ulua_interrupt()` calls | 14.3% | 23.2% | 5.3% | 2.4% | 1.5% |
| `%` through the cast `fmod` alias (not a builtin) | 9.6% | 2.1% | 9.7% | 0 | 0 |
| `math.fmod` builtin against native `%` | 27.5% | | 28.7% | | |
| Array helpers: integer check `__ulua_fmod(i, 1) == 0` | 0 | 0 | 0 | 52.7% | 36.9% |
| Array helpers: bounds compare and call | 0 | 0 | 0 | 26.2% | 27.7% |
| Comparison stored in a boolean | 3.6% | 5.2% | 8.8% | 3.7% | 3.0% |
| Slot form (entry temporaries, constants in registers, argument moves) | 17.2% | 56.9% | 39.6% | 5.9% | 14.8% |

Native `%` differs from JavaScript for negative operands, an infinite
divisor, `-0`, non-integers, and values above 2^53 (`-7 % 2`: Luau 1,
JavaScript -1), so it is correct only where both operands are proven
non-negative integers below 2^53. A builtin call (FASTCALL) fires no
interrupt; a shrink that makes a helper call a builtin must update
the §5.19 rule "a statement that always makes a Luau call ends the
count".

Shrink candidates, in order of measured share: (1) the integer check
of the array helpers (`math.floor(i) == i`: sort 378 to 229 ms, queen
1,902 to 1,417 ms); (2) the cast on the prelude aliases that blocks
FASTCALL (`typeof(math.fmod)` passes `luau-analyze` strict; fib-loop
1,331 to 1,199 ms); (3) the slot form; (4) cheaper interrupt points.

Round 1 record (40da51b, clean tree, 117 s for the full run): no
cell withheld.

| Workload | luau ms | ulua ms | ulua / luau |
|---|---:|---:|---:|
| fib-recursive | 59.2 | 185.0 | 3.13 |
| fib-loop | 112.4 | 863.4 | 7.68 |
| mandelbrot | 480.3 | 962.5 | 2.00 |
| primes | 84.5 | 700.6 | 8.29 |
| sort | 59.1 | 410.7 | 6.95 |
| tree | 31.5 | 107.9 | 3.42 |
| queen | 421.5 | 2036.0 | 4.83 |
| particles | 83.0 | 602.4 | 7.26 |
| callbacks | 100.5 | 452.1 | 4.50 |
| collect | 127.3 | 357.0 | 2.80 |

## Measurements (P6.S1)

Record at 5c314e2 (clean tree). `ulua / luau` before (40da51b) and
after: fib-recursive 3.13 to 3.04, fib-loop 7.68 to 7.55, mandelbrot
2.00 to 1.92, primes 8.29 to 7.61, sort 6.95 to 4.22, tree 3.42 to
3.26, queen 4.83 to 3.63, particles 7.26 to 5.44, callbacks 4.50 to
3.84, collect 2.80 to 2.60. 33 index trap edges equal the output and
trap kind before the change. The FASTCALL test is Red at d7e542f.
`gate.sh full` 173 s, `gate.sh timing` 378 s.

## Measurements (P6.2)

Record at a65b01d (clean tree), clock quantum 41 ns, every variant in
its noise gate. ns per call:

| Boundary | Floor | Ulua | Largest layer |
|---|---:|---:|---|
| host into script (`ulua_call`) | 24.6 | 110.3 | protected wrapper and export lookup, 41.4 ns (37.6%); Rust ABI, guards, and setup 38.6 ns (35.0%) |
| script into host (granted function) | 27.7 | 56.9 | Rust ABI, guards, and setup 23.9 ns (42.0%); host trampoline and call charge 9.3 ns (16.4%) |

The step and allocation charge is 2.2 ns of `ulua_call`. Shrink
candidates: the protected wrapper and export lookup of `ulua_call`,
and the Rust-side work of a host call. P6.2 changes no production
code; a shrink is a later slice.

## Measurements: slot form (before P6.S2)

Measurement only, at f5f026e: 110 hand-edited variants of the
generated Luau, each with the checksum of HEAD and `luau-analyze`
strict clean; instruction counts from `lua_singlestep`. Share of the
difference to hand-written Luau, each layer removed alone:

| Layer | fib-rec | primes | fib-loop | queen | particles |
|---|---:|---:|---:|---:|---:|
| Instructions per unit, HEAD (hand) | 24.5 (6.5) | 32.7 (6.1) | 25.2 (4.5) | 104.8 (14.8) | 54.7 (12.2) |
| Loop form (copies, tail flag, `repeat`) | 1.9% | 8.5% | 12.6% | 5.9% | 5.2% |
| Expression form (literal, comparison, call argument, operand, store), together | 54.0% | 22.9% | 43.4% | 10.1% | 13.4% |
| Entry temporaries as single-assignment locals, alone | 51.2% | 14.9% | -6.2% | 7.9% | 4.1% |
| Entry after the expression form | 0.7% | 1.2% | 0.5% | 2.1% | 2.9% |
| All slot layers | 53.7% | 34.9% | 55.8% | 18.2% | 17.4% |

The pass-to-pass spread is up to 8.3%; a share below 5% is not
resolved. Outside the slot form: `__ulua_interrupt()` lines 6 to 29%;
the array helpers up to 61% (queen, an upper bound that removes the
check). Order of slices: the loop form (small change), then the
expression form (largest share). The entry form adds little after
the expression form and changes the slot allocator; it is not
planned.

## Measurements (P6.S2)

Record at 529e818 (clean tree). `ulua / luau` after P6.S1 and after
P6.S2: fib-loop 7.55 to 6.68, primes 7.61 to 7.49, queen 3.63 to
3.50, particles 5.44 to 5.03, collect 2.60 to 2.42; the others hold
(fib-recursive 3.07, mandelbrot 1.93, sort 4.16, tree 3.32,
callbacks 3.87). `gate.sh full` 134 s, `gate.sh timing` 377 s.

## Measurements (P6.S3)

Record at 4f6f4a4 (clean tree). `ulua / luau` after P6.S2 and after
P6.S3: fib-recursive 3.07 to 1.75, fib-loop 6.68 to 2.91, mandelbrot
1.93 to 1.49, primes 7.49 to 5.44, sort 4.16 to 3.70, tree 3.32 to
2.82, queen 3.50 to 3.33, particles 5.03 to 4.55, callbacks 3.87 to
3.43, collect 2.42 to 2.21. Instructions: fib-recursive 24.5 to 12.5
per call (hand-written Luau 6.5); fib-loop 22.0 to 8.0 per inner
iteration (4.5). Largest statement 36 (bound 36); largest gap 49
(bound 579); largest ratio 5.80 (bound 6). `gate.sh full` 140 s,
`gate.sh timing` 366 s.

## Measurements: remaining overhead (after P6.S3)

Measurement only, at ca14d54 (hand-edited variants, checksums equal).
Share of the difference to hand-written Luau, each layer alone:

| Layer | primes | particles | sort | callbacks | queen | tree | collect |
|---|---:|---:|---:|---:|---:|---:|---:|
| Interrupt callback and charges | 15.8% | 18.6% | 10.0% | 6.7% | 11.4% | 25.3% | 17.1% |
| `__ulua_interrupt()` calls | 13.7% | 31.1% | 0 | 1.2% | 0 | 12.7% | 27.3% |
| of which the loop-head point | 11.3% | 16.2% | 0 | 2.0% | 1.6% | | 20.4% |
| `%` through `fmod` | 49.9% | 0 | 6.2% | 51.0% | 0 | 1.4% | 8.8% |
| `__ulua_equal` | 32.4% | | | 11.9% | 0 | 23.0% | 2.9% |
| Index check (no check: an upper bound) | | 46.9% | 85.1% | | 76.6% | | 12.2% |
| Index check inline as nil and floor tests | | 25.1% | 42.1% | | 39.8% | | 10.6% |
| Callback length check | | | | 12.5% | | | |
| Closure call per element | | | | 29.7% | | | |
| Slot residue | 5.3% | 10.0% | 4.0% | 5.4% | 6.7% | 34.5% | 16.4% |
| Counted loop as a numeric `for` | 0 | 15.6% | 0 | 0 | 11.5% | 0 | 14.4% |

Native `%` is valid only with a proof that the dividend is a
non-negative integer below 2^52 and the divisor a positive integer; a
runtime guard is slower (primes -45.8%). Next after P6.S4: condition
`||` and `&&` (queen reaches hand speed), counted loops as numeric
`for`, the receiver copies of the slot residue.

## Measurements (P6.4, measurement round)

Codex prototype, all production changes reverted (git status clean).
Cause: with step multiplier 200 the remark phase does not drain; the
spill-table barrier re-grays a new closure each iteration (traversal
2,816 to 2,928 bytes against a 2,048-byte work unit). The allocator
step charge does not repay GC debt; a refusal does not update
`totalbytes`. Candidates over 54 conditions:

| goal / step mul / step size | `limit:memory` of 54 | Largest peak | Largest §5.19 ratio | collect / tree ms |
|---|---:|---:|---:|---|
| 200 / 200 / 1 KiB (default) | 6 | 67,107,964 | 5.79 | 288.6 / 90.3 |
| 200 / 400 / 1 KiB | 0 | 1,137,356 | 5.75 | 289.7 / 92.3 |
| 400 / 400 / 1 KiB | 0 | 1,714,880 | 5.78 | 285.8 / 89.3 |
| 100 / 400 / 1 KiB | 0 | 975,212 | 7.84 | 297.4 / 99.5 |
| a full collection in the allocator before a refusal | 0 | 67,107,964 | 5.59 | 291.5 / 92.6 |
| goal 1000 | 18 | 67,107,964 | 5.95 | 275.9 / 84.0 |

Rule: goal 200, step multiplier 400, step size 1 KiB (`pipeline.md`
§6.5). The S3 fixture that doubles a string 25 times to reach
`limit:memory` peaks at 66,135,143 bytes under the rule and passes;
26 doublings reach the limit.

## Measurements (P6.4, P6.S4)

Record at ab0ab67 (clean tree). `ulua / luau` after P6.S3 and after
P6.S4: primes 5.44 to 4.04, sort 3.70 to 3.12, tree 2.82 to 2.34,
queen 3.33 to 2.70, particles 4.55 to 3.90, callbacks 3.43 to 3.18,
collect 2.21 to 2.11; fib-recursive 1.75 and fib-loop 2.93 hold;
mandelbrot 1.49 to 1.58 (737 to 781 ms): the moved loop-head point
dropped the header's reserved count, so a point ran in the condition
of each inner iteration (29 to 32 instructions); d3b1f0b restores the
count, and the record at d3b1f0b gives mandelbrot 1.48 (731 ms),
particles 3.69, queen 2.66, primes 4.27 (Luau 80.2 ms in this run),
the others within 2%.
P6.4: the 18-frame spill loop ends with `limit:steps` at 4 and 16 MiB
(`limit:memory` at ab0ab67's parent), peak 1,137,356 bytes, 709
cycles. Limit fixtures rebuilt: string doubling 25 passes and 26
reaches `limit:memory`; recursion 19,997 passes and 19,998 reaches
`limit:stack` (native equality removes a helper frame). `gate.sh
full` 155 s, `gate.sh timing` 370 s.

## Measurements (P6.S5)

Record at f83f4c9 (clean tree). `ulua / luau` after the P6.S4 fix
and after P6.S5: mandelbrot 1.48 to 1.16, fib-loop 2.89 to 2.24,
primes 4.27 to 3.26, particles 3.69 to 2.80, queen 2.66 to 2.44, sort
3.17 to 3.00, collect 2.11 to 2.04; fib-recursive 1.75, tree 2.34,
callbacks 3.15 hold. `CONDITION_NODES` 15. Largest gap 49, statement
36, ratio 5.85, capture ratio 3.54, analyzer ratio 11.26. `gate.sh
full` 149 s; timing witnesses 394 s.

## Measurements (P6.3)

`crates/ulua/tests/witnesses/p63_performance.rs` (gate timing, Debug
Rust, native O3): ten workloads at reduced sizes (Luau medians 7.9 to
17.2 ms), three runs, bound = largest ratio × 1.25: fib-recursive
2.19, fib-loop 2.88, mandelbrot 1.46, primes 3.44, sort 3.66, tree
2.98, queen 3.07, particles 3.52, callbacks 3.97, collect 2.56. Red
against a build with the compiler of 84b6118 (before P6.S3): 8 of 10
fail (callbacks and collect pass). Cost 4.7 s. `gate.sh timing` 375 s.

## P6 Phase Review

Cumulative review of 6cc1a27..5e96d10: 0 CRITICAL, 0 MAJOR, 2 MINOR.
No accepted program differs from `node` or from 6cc1a27 (361 random
differential programs, 120 index edges × forms, counted-loop bounds
NaN, ±Infinity, -0, non-integers, 2^53 ± k with step 2, overflowing
steps); largest interrupt gap 21 and statement 16 on 7 new shapes.

| Finding | Severity | Resolution |
|---|---|---|
| The spill fixpoint loop of `emit_with_exports` has no iteration bound; termination rests on a comment | MINOR | an iteration cap with an `internal` diagnostic and a test that builds a non-monotone plan |
| `gate.sh sanitize` at the close: UBSan "nan is outside the range of representable values of type 'int'" at the pinned `lvmexecute.cpp:779`; the P6.S4 inline element read indexed `a[i + 1]` before its test, so NaN, an infinity, or a magnitude at or above 2^31 reached `int(indexd)` | MAJOR | `pipeline.md` Rev 51: the helper's test runs before the access; the async job queue indices are bounded; sort 3.00 to 3.70, queen 2.44 to 3.22, particles 2.80 to 3.39 |
| The P6.3 bounds come from 3 runs × 1.25; a loaded machine can move a median-of-7 ratio past the margin | MINOR | open: a flake risk only; the test runs under the timing lock in `gate.sh timing` |

## P6 close

Fixes after the Phase Review: the spill fixpoint has an iteration cap
(bindings + 1, an `internal` diagnostic; largest observed 3); the
index test runs before the table access (Rev 51); the async job queue
indices are bounded; a refused allocation inside a GC stack shrink is
kept as the stop reason (it gave `async:pending` for `a64` at k=5
under the P6.4 settings); a write scopes its receiver (analyzer ratio
of P22-array-index-writes 14.05 to 7.89). The P6.3 bounds are
re-derived after Rev 51: sort 4.54, queen 4.01, particles 4.42,
primes 3.62, others within 0.1 of before. `gate.sh sanitize` excludes
`ulua_bench` (its hosts link the unsanitized VM; gate full runs its
tests). Close gates: `gate.sh full` 124 s, `gate.sh sanitize` 228 s,
timing witnesses 376 s. Running tasks at the close: none.

Final record at 7cca876 (clean tree), `ulua / luau`: mandelbrot 1.16,
fib-recursive 1.75, collect 2.08, fib-loop 2.31, tree 2.32, queen
3.07, callbacks 3.16, primes 3.40, particles 3.51, sort 3.69 (P6.1
start: 2.00 to 8.29).

## Measurements: roblox-ts comparison (after P6 close)

Measurement only; no production change; `git status` clean after the
run. Subject: [roblox-ts](https://github.com/roblox-ts/roblox-ts)
v3.0.0 at commit `68648b0c`, built locally, `--type model`. Its output
of the ten Ulua workloads replaced `workloads/luau/*.luau` for one
`bench --only` run per workload, then `git checkout` restored them.
The `bench` binary is the build of 7cca876 (the crates of HEAD differ
only in the record files). Same machine and session as the hand
baseline below; checksums equal; no cell withheld.

Source changes for roblox-ts, mechanical: `null` to `undefined` (tree,
collect; roblox-ts rejects `null`); `.length` to `.size()` (collect);
a local named `next` to `nxt` (collect, fib-loop, mandelbrot; a
reserved identifier in roblox-ts). The output keeps the module table;
a wrapper returns `main`, outside the timed region.

| Workload | hand Luau ms | roblox-ts ms | Ulua ms | roblox-ts / hand | Ulua / hand | Ulua / roblox-ts |
|---|---:|---:|---:|---:|---:|---:|
| fib-recursive | 61.0 | 61.3 | 106.9 | 1.01 | 1.75 | 1.74 |
| fib-loop | 115.4 | 196.7 | 260.5 | 1.71 | 2.26 | 1.32 |
| mandelbrot | 481.6 | 481.4 | 573.6 | 1.00 | 1.19 | 1.19 |
| primes | 84.7 | 97.1 | 262.7 | 1.15 | 3.10 | 2.71 |
| sort | 59.8 | 59.5 | 217.7 | 0.99 | 3.64 | 3.66 |
| tree | 31.5 | 58.9 | 73.5 | 1.87 | 2.33 | 1.25 |
| queen | 421.8 | 486.1 | 1331.7 | 1.15 | 3.16 | 2.74 |
| particles | 83.2 | 96.2 | 287.8 | 1.16 | 3.46 | 2.99 |
| callbacks | 100.1 | 133.1 | 319.1 | 1.33 | 3.19 | 2.40 |
| collect | 127.0 | 172.1 | 266.5 | 1.36 | 2.10 | 1.55 |

The emitted forms of roblox-ts, read from its output:

- `%` is the native Luau `%`. It differs from `node` for a negative
  operand (C4.3).
- An element access is `a[i + 1]` with no index check.
- No interrupt point and no step or allocation charge.
- A `for` loop is a numeric `for` only for an integer literal start
  and an integer literal bound. Otherwise it is `while true` with a
  `_shouldIncrement` flag (fib-loop, primes, collect: slower than
  hand Luau).
- A class constructs through `Class.new(...)`, `setmetatable`, and
  `self:constructor(...) or self` (tree: 1.87 against hand Luau).
- `map`, `filter`, and `reduce` expand inline to loops; each element
  still calls the callback.

Consequence: the gap of Ulua to roblox-ts on sort, particles, and
queen is the index check and the interrupt cost (invariant 2). On
primes and callbacks the largest part is `%` through `fmod` (the
remaining-overhead table above: 49.9% and 51.0%). Next: P6.S6.

## Measurements (P6.S6)

Record at 535f2fe (clean tree). `ulua / luau` at 7cca876 and at
535f2fe: primes 3.40 to 1.19 (263.0 to 101.7 ms), fib-loop 2.31 to
2.08, sort 3.69 to 3.53, callbacks 3.16 to 3.12; fib-recursive 1.75,
mandelbrot 1.16 to 1.15, tree 2.32 to 2.33, queen 3.07 to 3.12,
particles 3.51 to 3.49, collect 2.08 to 2.06. Largest IQR 3.22% of
the median (fib-loop / luau); no cell withheld.
Native sites: primes `n % d` and `(count + 1) % 4294967296`; fib-loop
`i % 40`; collect `i % 2`; the LCG of sort and callbacks. Largest
round count: corpus 6, workloads 8 (fib-loop: the `a`, `b`, `next`
component reaches the limit and loses its facts; `i` keeps its fact).
Shadow check: corpus 79, workloads 10, generator 361 (140 native
sites), no stop. `cargo test -p ulua_compiler` 218 passed; `gate.sh
full` 139.65 s; `gate.sh timing` 446.35 s: §5.19 largest 5.856,
analyzer largest 11.196, fact-rounds growth 4.454 and 4.161 (54, 218,
873 components at the limit), P6.3 within every bound.

Review of the first form: a write that read an unset member of its
component never joined, so a member kept an integer fact without that
write (Rev 54). The IR-built test `unset_member_invalidates_the_whole_component`
fails before the fix (2 fmod sites expected, 0 found).

## P6.S6 Phase Review

Review of d34a194..b00ff4c: 0 CRITICAL, 0 MAJOR, 5 MINOR. No native
`%` differs from `node`: 181 + 80 generated programs (about 2,200
functions, literals near 2^53) equal `node`; 201 generated programs
with 9,260 native sites under the shadow check, no stop; the proof of
§5.25 checked by hand against `luai_nummod`, `LOP_MOD`, `LOP_MODK`,
and the constant fold at c54f558.

| Finding | Severity | Resolution |
|---|---|---|
| The fact pass walks the IR by recursion with no depth counter and indexes maps with `[]` (panic on a missing key); §3 item 4 | MINOR | Rev 55: work lists or a depth counter; a missing node is `internal` |
| No gate runs the shadow check; the instruction-bound test counts `" % "` and fails under it | MINOR | Rev 55: `gate.sh full` runs the corpus and workload shadow tests by name |
| The shadow option is read from the environment in each `Emitter::new`; a change during one compile can omit the helper | MINOR | Rev 55: one compilation reads the option once |
| Thresholds per function: `i % 2` to `i % 8` in one loop gave 0 native of 7 (a cost that the source of one `%` does not show) | MINOR | Rev 55: thresholds per binding |
| "system" names the solved unit in the contract and the function graph in the code | MINOR | Rev 55: "component" in the contract; the code type is renamed |

Close of the review (code of the fixes, `gate.sh` change): the walks
take the depth counter of §3 item 4 (`limit:nesting-depth`; a missing
node is `internal`); thresholds per binding (`i % 2` to `i % 8` in one
loop: 7 of 7 native, 0 before); one compilation reads the shadow
option once; the graph type is `Graph`. Native sites per workload are
unchanged; largest round count: corpus 6 to 4, workloads 8 to 6.
`cargo test -p ulua_compiler` 224 passed. `gate.sh full` PASS with the
shadow steps (corpus 3 s, workloads 10 s). `gate.sh timing`: 14 of 15
passed; `p25_witness_time` failed with `P22-array-wide-function-product`
at 12.023 against 12 (a shape with no `%`: function types only); the
test alone gave 11.588 (159 s). The same shape gave 11.196 and 11.662
in the two earlier timing runs of this slice. Fact-rounds growth 4.469
and 4.192; §5.19 largest 5.828; P6.3 within every bound.

Open: the analyzer ratio of `P22-array-wide-function-product` moves
from 11.2 to 12.0 between runs (its 4 KiB base is 22 ms), so the bound
12 can fail on noise (MINOR, flake risk).

## Measurements: Windows x86_64 (benchmarks.md Rev 5)

Failure: `gate.sh timing` on Windows x86_64 at 7e08e23 failed
`ten_workload_performance_ratios`: mandelbrot 1.594 > 1.46; a rerun
gave 1.589. The same gate gave `async-reject-resume` 6.17 > 6 in
`p55_ratio`; a rerun gave 5.195 (noise, MINOR, as the p63 flake risk
above).

Three runs at c8cb050, `p63_performance`, `ulua / luau`:

| Workload | Run 1 | Run 2 | Run 3 | macOS max | Bound |
|---|---:|---:|---:|---:|---:|
| fib-recursive | 1.660 | 1.583 | 1.579 | 1.739 | 2.18 |
| fib-loop | 1.301 | 1.293 | 1.361 | 2.275 | 2.85 |
| mandelbrot | 1.585 | 1.644 | 1.564 | 1.160 | 1.46 to 2.06 |
| primes | 1.420 | 1.406 | 1.398 | 2.889 | 3.62 |
| sort | 3.401 | 3.351 | 3.142 | 3.628 | 4.54 |
| tree | 1.927 | 2.040 | 2.083 | 2.301 | 2.88 |
| queen | 4.121 | 4.157 | 4.126 | 3.207 | 4.01 to 5.20 |
| particles | 2.176 | 2.194 | 2.183 | 3.533 | 4.42 |
| callbacks | 1.614 | 1.541 | 1.669 | 3.176 | 3.98 |
| collect | 1.695 | 1.713 | 1.651 | 2.082 | 2.61 |

- Eight workloads have a lower ratio on Windows. mandelbrot and queen
  have a higher ratio. The cause is not measured.
- queen gave 3.557 in the earlier rerun of the same session and 4.12
  to 4.16 in these three runs.

Consequence: a bound from one target does not hold on another. Each
bound is the largest of the three runs over each measured target,
times 1.25.

## Exit checklist (P6.1)

- [x] `crates/ulua_bench`: the runner, the two subjects, the ten
  workloads in Ulua and Luau, the parameters in its README.
- [x] Checksums equal in both subjects.
- [x] The timing method of §3: samples, the IQR noise gate, the
  warm-up floor with reported warm-up, a fresh process per cell,
  `--only` writes no record.
- [x] One full run on macOS arm64 writes `results.json` and the table.
- [x] Workspace gates stay green; the crate is not in `gate.sh full`
  except a build and a unit test of the runner's statistics.
- [x] Phase Review of the slice.
