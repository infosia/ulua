# Corpus contract

Status: Rev 0, 2026-10-11.

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
// tsc: accepts | errors <TSnnnn>[, <TSnnnn>...]
// expected-error: <rule id>: <text>        (reject only)
// trap: <kind>                             (trap only)
```

- `tsc` is measured, never assumed. An accept or trap entry always
  states `tsc: accepts`. A reject entry states the result of `tsc` on
  that file alone with the project options.
- `rule` names collision ids (`C1`, `C2`, ...) of `collisions.md`.
- `expected-error` starts with one id of the `rule` header. The
  compiler diagnostic must carry that id. Until P1 defines diagnostic
  codes, the gate checks only the header.
- `trap` names one trap kind of §6.

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
- `node`: each accept entry writes exactly its golden, byte for byte,
  and does not throw. For a trap entry, the `node` output starts with
  the golden lines before the `trap:` line. Then `node` must write more
  lines or throw. After that point `node` continues with a value
  outside the static type, and the gate does not compare it.
- The Luau witnesses join at P1 (`plan.md` §5).

## 5. Golden changes

A golden states a reviewed result. Never change a golden to match a
failing implementation. A golden change needs a contract change or a
recorded defect of the golden, in the same commit.

## 6. Trap kinds

| Kind | Rule |
|---|---|
| `index-out-of-range` | collisions.md C6 |

Resource limit traps (depth, time, memory; `CLAUDE.md` invariant 2) join
this table at P1 with their own entries. The `node` comparison does not
apply to them.
