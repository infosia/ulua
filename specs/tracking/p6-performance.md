# P6 performance

Status: open; P6.1, P6.S1, P6.2, P6.S2, P6.S3, P6.4, P6.S4, P6.S5, P6.3 COMPLETE 2026-10-16. Contract: `blocks/benchmarks.md` Rev 1 (Luau and Ulua only, owner decision 2026-10-16).

## Slices

| Slice | Family | Exit |
|---|---|---|
| P6.1 | Luau and Ulua suite (`benchmarks.md` §1 to §4) | Both subjects give the same checksum on all ten workloads; the noise gate passes or withholds; `results.json` and the README table are written by one full run |
| P6.2 | Boundary price (§5) | ns per call for both boundaries, each layer's share, a named shrink candidate or "the floor"; no production change |
| P6.3 | Performance gate (§6) | A ratio test in `gate.sh timing` with a bound from the P6.1 record; Red with a seeded slowdown |
| P6.4 | GC pacing against `max_memory_bytes` (P5 carry-over) | `pipeline.md` §6.5 settings; the 18-frame spill loop ends with `limit:steps` at 4 and 16 MiB; the §5.19 ratio tests pass |
| P6.5 | Reload | Contract first, at the slice start |

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
