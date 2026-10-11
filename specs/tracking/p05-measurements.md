# P0.5 measurements

Status: open. M4 and M5 are measured. M1, M2, and M3 are in progress.

Every prototype of this round is outside the repository. No production
change lands.

## Exit checklist

- [ ] M1 build root: numbers and a decision in `plan.md` §8.
- [ ] M2 Rust and the Luau C API: numbers and a decision.
- [ ] M3 parser depth: numbers and a decision (D7).
- [x] M4 number to string: numbers and a decision (D9).
- [x] M5 witness cost: numbers and a decision (D10).

## Setup

- `node` v24.18.0, TypeScript 5.9.2.
- Luau CLI and `luau-analyze` built in Release from the revision in
  `luau-revision.txt` (0.738), Unix Makefiles.

## M4 — number to string

Probe: `tools/probes/number-format.mjs` and `.luau`. A seeded xorshift32
generator gives the same values in both. Categories, 20,000 values each:
random bit patterns (`bits`), integers up to 2^53 with both signs
(`int`), short decimals `k / 10^d` (`dec`), and values near the format
boundaries `1e-6` and `1e21` (`edge`). Then 19 special values.

Commands:

```text
node tools/probes/number-format.mjs > node.txt
luau tools/probes/number-format.luau > raw.txt
luau tools/probes/number-format.luau -a lowered > lowered.txt
paste -d'|' node.txt raw.txt | awk -F'|' '$1!=$2' | wc -l
```

| Sample | Values | `tostring` differs | Lowering differs |
|---|---|---|---|
| 20,000 per category | 80,019 | 9,294 (117 bits, 3,163 dec, 6,007 edge, 7 special) | 0 |
| 200,000 per category | 800,019 | 93,358 | 0 |

Each `tostring` difference is one of: `nan`, `inf`, `-inf`, `-0`, or an
exponent with a leading zero (`1e-07`). The `int` category has no
difference. The digits agree on every value: both print the shortest
round-trip form.

The lowering (the `lowered` function in the probe): NaN to `NaN`, the
infinities to `Infinity` and `-Infinity`, zero (both signs) to `0`,
else `tostring` with `e([+-])0*(%d)` replaced by `e%1%2`.

Decision D9: lower a number in a template substitution and in
`String(n)` to this function (collisions.md C4.5).

## M5 — witness cost

| Witness | Input | Wall time |
|---|---|---|
| `luau-analyze`, one process per file | 12 probe files, 960 lines | 0.10 s (about 8 ms per file) |
| `luau-analyze`, one process | the same 12 files | 0.01 s |
| `luau-analyze`, one process | one module, 35,002 lines, 5,000 typed functions | 0.25 s |
| `node`, process start | `node -e 0`, 10 runs | 0.29 s (29 ms per run) |
| corpus witness (`tools/witness/corpus.mjs`) | 96 entries, two `tsc` programs, 27 node imports in one process | 0.49 s |
| `tsc -p tsconfig.json --noEmit` | prelude, accept and trap entries | 0.56 s |

`luau-analyze` exit status: 1 with a `TypeError`; 0 with lint warnings
only (`LocalUnused`, `FunctionUnused`, `IntegerParsing`).

Decision D10: the gate runs `luau-analyze` once over every generated
module, and `node` once over every entry in one process. The Luau
witness fails on exit status 1. Lint warnings do not fail it.

## Gaps

- M5 measures hand-written Luau. P1 measures the generated corpus.
