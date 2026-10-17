# Corpus contract

Status: Rev 11, 2026-10-14 (Rev 0 the same day; Rev 1 adds the `phase`
header and the Luau witness rules for P1; Rev 2 adds the
`js-comparable` header, CLAUDE.md invariant 5; Rev 3 adds slices to
`phase`; Rev 4 adds the `equality` `tsc` header, D17; Rev 5 adds the `never`
codes to it; Rev 6 renames it to `exception`, P2.5 review; Rev 7: any code that names `never`,
P2.5 review 11; Rev 8: `luau-analyze -j1`, P2.6 review 3; Rev 9: the
`grants` header and the host fixtures, P3; Rev 10: `async` `main`, unhandled
rejection, P4.1; Rev 11: the `hostLater` fixture, P4.3).

## 1. Layout

| Directory | Content | Golden |
|---|---|---|
| `corpus/accept/` | Programs that the compiler accepts and runs to completion | `<name>.expected`: the exact standard output |
| `corpus/reject/` | Programs that the compiler rejects | the `expected-error` header |
| `corpus/trap/` | Programs that the compiler accepts and that trap at run time | `<name>.expected`: the output before the trap, then one line `trap: <kind>` |

A name is `<letter><two or more digits>-<slug>`: `a` for accept, `r` for
reject, `t` for trap. A number is never reused. A retired entry keeps
its record in the rule that named it, written `retired:<name>`.

## 2. Program form

A corpus program is one module. It exports `function main(): void` or
`async function main(): Promise<void>`. The harness calls `main` once;
the call includes the job drain (collisions.md C19.6). The `node`
runner awaits the promise of an `async` `main`. A program writes output only through
`print(text: string): void` from the prelude. Each call writes `text`
and one line feed.

## 3. Header

Each entry starts with `//` header lines, in this order:

```text
// corpus: <dir>/<name>
// purpose: <one sentence>
// rule: <collision ids, comma-separated>
// phase: P1 | P2 | ... | P7, with an optional slice: P2.1
// grants: <fixture names, comma-separated>   (optional)
// tsc: accepts | errors <TSnnnn>[, <TSnnnn>...]
// js-comparable: yes | no <collision id>    (accept and trap only)
// expected-error: <rule id>: <text>        (reject only)
// trap: <kind>                             (trap only)
```

- `tsc` is measured, never assumed. An accept or trap entry states
  `tsc: accepts`, or `tsc: exception <codes>` when `tsc` gives
  those codes only where CLAUDE.md invariant 3 allows (collisions.md
  C7, C16). A reject entry states the result of `tsc` on
  that file alone with the project options.
- `rule` names collision ids (`C1`, `C2`, ...) of `collisions.md`.
- `phase` names the phase from which the Luau witnesses check the entry
  in full (§4).
- `expected-error` starts with one id of the `rule` header. The
  compiler diagnostic code must equal that id (§4).
- `trap` names one trap kind of §6.
- `grants` names fixtures of §7. The harness grants exactly these to
  the compilation, in header order. Without the header, it grants
  none. The `tsc` witness checks the entry with the declarations of
  these grants in the format of `abi.md` §6.3.
- `js-comparable: no <id>` names a rule of the `rule` header. That
  rule of `collisions.md` names the divergence, with the measured
  outputs of both sides.

## 4. Witness rules

`tools/witness/corpus.mjs` checks each rule below in one process.

- Format: the header keys and their order, the `corpus` header, the
  file name and its directory, the `expected-error` id, the trap kind,
  and no reused number. A `retired:` name in `collisions.md` keeps its
  number reserved.
- Citations: each id in a `rule` header is a rule of
  `collisions.md`. Each rule names exactly the entries that cite it.
- `tsc`: `tsconfig.json` has no option errors. The entries form one
  group per `grants` value (none included). One program per group,
  over the prelude, the host declarations of the group (`abi.md`
  §6.3, written by the witness), and every accept and trap entry of
  the group, reports zero errors, except
  the codes of an `exception` header: each such diagnostic lies on a
  `===`, `!==`, `==`, or `!=` expression whose two operands have one
  `number`, `string`, or `boolean` type (literal types widened; an
  operand narrowed to `never` counts as either) for TS2367 and TS2845;
  each other diagnostic names the type `never` in its message (`type 'never'`); and the codes equal the codes of the header. One
  program per group over the prelude, the host declarations, and every
  reject entry of the group reports, per file, the codes that its
  header states.
- `node`: the gate runs each entry with `js-comparable: yes`. Each such
  accept entry writes exactly its golden, byte for byte,
  and does not throw. For a trap entry, the `node` output starts with
  the golden lines before the `trap:` line. Then `node` must write more
  lines or throw. After that point `node` continues with a value
  outside the static type, and the gate does not compare it.
- The Luau witnesses (`tools/witness/luau.mjs`) run when `Cargo.toml`
  exists. They build `ulua-corpus` (`pipeline.md` §8), run every entry
  in one process, and run `luau-analyze` once over all generated Luau.
  The current phase is the constant `PHASE` in that file, with its
  slice. `P2.1` comes before `P2.2`; `P2` without a slice comes after
  every P2 slice.
- An entry whose phase is the current phase or earlier:
  - accept: status `ok`, and the output lines equal the golden lines;
  - reject: status `rejected`, and the code equals the
    `expected-error` id;
  - trap: status `failed`, the output lines equal the golden lines
    before the `trap:` line, and the code is `trap:<kind>`.
- An entry of a later phase:
  - accept and trap: status `rejected` with the code `unsupported`.
    If the entry passes in full, the witness fails and asks for a
    change of its `phase` header;
  - reject: status `rejected`, with any code.
- No entry gives status `internal`.
- `luau-analyze` exits with status 0 over the generated Luau of every
  compiled entry (D10). Every call of `luau-analyze`, in the witness
  and in a test, passes `-j1`. The pinned Luau writes the shared
  `Frontend::iceHandler.moduleName` from each worker thread
  (`Analysis/src/Frontend.cpp`), and a parallel run can die by a
  signal with no output: 96 of 12,000 parallel runs, 0 of 12,000 with
  `-j1`. A failure reports the exit status and the signal.

## 5. Golden changes

A golden states a reviewed result. Never change a golden to match a
failing implementation. A golden change needs a contract change or a
recorded defect of the golden, in the same commit.

## 6. Trap kinds

| Kind | Rule |
|---|---|
| `index-out-of-range` | collisions.md C6 |
| `array-modified` | collisions.md C6.9 |
| `map-modified` | collisions.md C6.10 |
| `string-boundary` | collisions.md C5 |
| `invalid-argument` | collisions.md C5 |
| `uncaught-exception` | collisions.md C17.8, C19.7 (an unhandled rejection) |

Resource limit traps (depth, time, memory; `CLAUDE.md` invariant 2) join
this table at P1 with their own entries. The `node` comparison does not
apply to them.

## 7. Host fixtures

The native harness (`pipeline.md` §8) and the `node` runner implement
each fixture below with the same result. The `node` runner passes each
granted fixture to the program as a function value, as it passes
`print`.

| Name | Signature | Result |
|---|---|---|
| `hostAdd` | `(a: number, b: number): number` | `a + b` |
| `hostConcat` | `(a: string, b: string): string` | `a + b` |
| `hostNot` | `(b: boolean): boolean` | `!b` |
| `hostLog` | `(text: string): void` | writes one output line, `log:` then `text` |
| `hostLater` | `(n: number): Promise<number>` (async, `abi.md` §6.4) | a request; the harness settles it with `n` |

The harness settles the requests of `hostLater` one at a time, in the
order of their start: after `main` and its drain, while a request is
open, it completes the oldest one and pumps one settle. The `node`
runner keeps the same queue: `hostLater` returns a pending promise and
queues its resolver; after `main`, the runner resolves the oldest one
and waits one macrotask (`setImmediate`), while the queue is not
empty. Each settle is one host-started run, followed by a drain (P4
Step 0 M6). An `async` `main` that is pending after the last settle
fails with `async:pending`.
