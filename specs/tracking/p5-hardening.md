# P5 hardening

Status: open; P5.1 COMPLETE 2026-10-14. Decisions: D28, D29 (`plan.md` §8).

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

## Exit checklist (P5.2)

- [ ] pipeline.md §9 implemented: the `ulua_fuzz` crate, the four
  targets, the invariants, deterministic seeds, reduction, and the
  findings directory.
- [ ] A direct test per invariant: a seeded violation (test-only) is
  reported as a failure, so a clean run is evidence.
- [ ] The smoke runs in `sh tools/gate.sh sanitize`, with their cost.
- [ ] One hour per target on the sanitizer build with zero open
  findings; each finding became a test before its fix.
- [ ] Phase Review of the slice with no open CRITICAL or MAJOR finding.

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
