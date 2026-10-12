# Corpus contract

Status: Rev 3, 2026-10-12 (Rev 0 the same day; Rev 1 adds the `phase`
header and the Luau witness rules for P1; Rev 2 adds the
`js-comparable` header, CLAUDE.md invariant 5; Rev 3 adds slices to
`phase`).

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

A corpus program is one module. It exports `function main(): void`. The
harness calls `main` once. A program writes output only through
`print(text: string): void` from the prelude. Each call writes `text`
and one line feed.

## 3. Header

Each entry starts with `//` header lines, in this order:

```text
// corpus: <dir>/<name>
// purpose: <one sentence>
// rule: <collision ids, comma-separated>
// phase: P1 | P2 | ... | P6, with an optional slice: P2.1
// tsc: accepts | errors <TSnnnn>[, <TSnnnn>...]
// js-comparable: yes | no <collision id>    (accept and trap only)
// expected-error: <rule id>: <text>        (reject only)
// trap: <kind>                             (trap only)
```

- `tsc` is measured, never assumed. An accept or trap entry always
  states `tsc: accepts`. A reject entry states the result of `tsc` on
  that file alone with the project options.
- `rule` names collision ids (`C1`, `C2`, ...) of `collisions.md`.
- `phase` names the phase from which the Luau witnesses check the entry
  in full (§4).
- `expected-error` starts with one id of the `rule` header. The
  compiler diagnostic code must equal that id (§4).
- `trap` names one trap kind of §6.
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
- `tsc`: `tsconfig.json` has no option errors. One program over the
  prelude and every accept and trap entry reports zero errors. One
  program over the prelude and every reject entry reports, per file,
  the codes that its header states.
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
  compiled entry (D10).

## 5. Golden changes

A golden states a reviewed result. Never change a golden to match a
failing implementation. A golden change needs a contract change or a
recorded defect of the golden, in the same commit.

## 6. Trap kinds

| Kind | Rule |
|---|---|
| `index-out-of-range` | collisions.md C6 |
| `array-modified` | collisions.md C6.9 |

Resource limit traps (depth, time, memory; `CLAUDE.md` invariant 2) join
this table at P1 with their own entries. The `node` comparison does not
apply to them.
