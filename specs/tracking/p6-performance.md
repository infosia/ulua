# P6 performance

Status: open; P6.1 COMPLETE 2026-10-16. Contract: `blocks/benchmarks.md` Rev 1 (Luau and Ulua only, owner decision 2026-10-16).

## Slices

| Slice | Family | Exit |
|---|---|---|
| P6.1 | Luau and Ulua suite (`benchmarks.md` §1 to §4) | Both subjects give the same checksum on all ten workloads; the noise gate passes or withholds; `results.json` and the README table are written by one full run |
| P6.2 | Boundary price (§5) | ns per call for both boundaries, each layer's share, a named shrink candidate or "the floor"; no production change |
| P6.3 | Performance gate (§6) | A ratio test in `gate.sh timing` with a bound from the P6.1 record; Red with a seeded slowdown |
| P6.4 | GC pacing against `max_memory_bytes` (P5 carry-over) | A measured rule: either the instance sets the GC goal, or `pipeline.md` §6.5 names the bound with the measured worst case |
| P6.5 | Reload | Contract first, at the slice start |

Shrink slices follow the `ulua / luau` column of P6.1, each with a
stated problem and evidence (principle 13). They run before P6.3, so
the gate bound comes from the record after the shrinks.

| Slice | Family | Exit |
|---|---|---|
| P6.S1 | Builtin aliases and the index check (`pipeline.md` §5.20) | Each builtin alias compiles to `FASTCALL`; the trap set of the array helpers is unchanged (tests at each trap edge); the §5.19 tests pass; a new full record |

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
