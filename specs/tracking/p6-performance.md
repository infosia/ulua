# P6 performance

Status: open. Contract: `blocks/benchmarks.md` Rev 1 (Luau and Ulua only, owner decision 2026-10-16).

## Slices

| Slice | Family | Exit |
|---|---|---|
| P6.1 | Luau and Ulua suite (`benchmarks.md` §1 to §4) | Both subjects give the same checksum on all ten workloads; the noise gate passes or withholds; `results.json` and the README table are written by one full run |
| P6.2 | Boundary price (§5) | ns per call for both boundaries, each layer's share, a named shrink candidate or "the floor"; no production change |
| P6.3 | Performance gate (§6) | A ratio test in `gate.sh timing` with a bound from the P6.1 record; Red with a seeded slowdown |
| P6.4 | GC pacing against `max_memory_bytes` (P5 carry-over) | A measured rule: either the instance sets the GC goal, or `pipeline.md` §6.5 names the bound with the measured worst case |
| P6.5 | Reload | Contract first, at the slice start |

Shrink slices follow P6.2 and the `ulua / luau` column of P6.1, each
with a stated problem and evidence (principle 13).

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

## Exit checklist (P6.1)

- [x] `crates/ulua_bench`: the runner, the two subjects, the ten
  workloads in Ulua and Luau, the parameters in its README.
- [x] Checksums equal in both subjects.
- [ ] The timing method of §3: samples, the IQR noise gate, the
  warm-up floor with reported warm-up, a fresh process per cell,
  `--only` writes no record.
- [x] One full run on macOS arm64 writes `results.json` and the table.
- [ ] Workspace gates stay green; the crate is not in `gate.sh full`
  except a build and a unit test of the runner's statistics.
- [ ] Phase Review of the slice.
