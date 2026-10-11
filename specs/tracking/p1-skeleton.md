# P1 vertical skeleton

Status: open. Contracts: `pipeline.md` Rev 0, `abi.md` Rev 0,
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

- [ ] `sh tools/gate.sh full` passes with the Luau witnesses.
- [ ] Every function of `abi.md` has a direct unit test; the layout test
  passes.
- [ ] `pipeline.md` §7 values measured and pinned by tests.
- [ ] P0 carry-over: the P1 Phase Review verifies C12.6 and C16.4 (`plan.md` §9 Rev 2).
- [ ] P0 carry-over N3 (`a25`), N4 (`a26`), N5 (surface.md Rev 5).
- [ ] Phase Review with no open CRITICAL or MAJOR finding.

## Measurements

## Failures before the fix

## Gaps
