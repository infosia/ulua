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

## Exit checklist (P6.1)

- [ ] `crates/ulua_bench`: the runner, the two subjects, the ten
  workloads in Ulua and Luau, the parameters in its README.
- [ ] Checksums equal in both subjects.
- [ ] The timing method of §3: samples, the IQR noise gate, the
  warm-up floor with reported warm-up, a fresh process per cell,
  `--only` writes no record.
- [ ] One full run on macOS arm64 writes `results.json` and the table.
- [ ] Workspace gates stay green; the crate is not in `gate.sh full`
  except a build and a unit test of the runner's statistics.
- [ ] Phase Review of the slice.
