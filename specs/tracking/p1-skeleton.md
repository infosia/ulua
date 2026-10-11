# P1 vertical skeleton

Status: open. Contracts: `pipeline.md` Rev 1, `abi.md` Rev 0,
`corpus.md` Rev 1, `surface.md` Rev 5, `collisions.md` Rev 12.

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

## Exit checklist

- [x] `sh tools/gate.sh full` passes with the Luau witnesses.
- [ ] Every function of `abi.md` has a direct unit test; the layout test
  passes.
- [x] `pipeline.md` §7 values measured and pinned by tests.
- [ ] P0 carry-over: the P1 Phase Review verifies C12.6 and C16.4 (`plan.md` §9 Rev 2).
- [ ] P0 carry-over N3 (`a25`), N4 (`a26`), N5 (surface.md Rev 5).
- [ ] Phase Review with no open CRITICAL or MAJOR finding.

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

## Failures before the fix

## Gaps

- The `functions` input of `M heap` takes 502 ms in Release at 65,536
  bytes. The other inputs take at most 69 ms. The Phase Review checks
  whether this cost grows faster than the source size.
- Depth 48 admits 15 nested loops or `if` statements. P6 measures
  whether a lower stack cost per level raises it.
- The absolute Luau ceiling of template parts is unknown.
