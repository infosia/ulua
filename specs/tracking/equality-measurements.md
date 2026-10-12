# Equality measurements (type-based equality proposal)

Status: measured 2026-10-12. Measurement round (CLAUDE.md step 0): a
prototype in an isolated worktree at commit `4dfd8b3`, reverted. No
contract changed. The proposal under test: equality acceptance by
operand types only (identical `number`, `string`, or `boolean` types for
`===`, `!==`, `==`, `!=`; mixed types, function equality, and array
literal operands rejected; array identity kept; no wide-operand rule,
no tested references, `NaN` operands accepted). Rules it would change:
CLAUDE.md invariant 3, `plan.md` D15, `collisions.md` C7.

Tools: TypeScript 5.9.2 with the prelude and `--strict`; `node`
v24.18.0; the pinned Luau.

## Fixtures

Baseline is the current C7; prototype is the proposal. Each accepted
prototype run equals `node`, and its generated Luau passes
`luau-analyze` with no diagnostic.

| Fixture | Baseline | Prototype | `tsc` |
|---|---|---|---|
| boolean parameters `a === b`, `a !== b`, `a == b`, `a != b` | C7 | accepted | none |
| `true === false` | C7 | accepted | TS2367 |
| `const a = 1; const b = 2; a === b`; string constants | C7 | accepted | TS2367 |
| `if (x === 1) { ... x === 2 ... }` | C7 | accepted | TS2367 |
| a comparison through a boolean condition alias | C7 | accepted | TS2367 |
| a comparison in a closure created after a test | C7 | accepted | TS2367 |
| `NaN === NaN`, `x === NaN` | C7 | accepted | TS2845 |
| a narrowed boolean `let` | C7 | accepted | TS2367 |
| loops, negation, parentheses, an unreachable branch | C7 | accepted | TS2367 |
| boolean operands with side effects (order, single evaluation) | C7 | accepted, equal to `node` | none |
| an alias with 9 tested references | C7 | accepted | none |
| number and string operands | C7 | C7 | TS2367 |
| function equality | C7 | C7 | none |
| array identity | accepted | accepted | none |
| array literal operand | C7 | C7 | TS2839 |
| array element types that differ | C7 | C7 | TS2367 |
| an equality diagnostic plus an unrelated type error | C7 | C1 | TS2322, TS2367 |

Corpus entries that change from C7 rejection to acceptance, each equal
to `node`: `r17` (no `tsc` code), `r70`, `r71`, `r72`, `r74`, `r76`,
`r77`, `r79`, `r80` (TS2367), `r73` (TS2845), `r83` (no `tsc` code).

Tooling: of 30 programs that the prototype accepts and the baseline
rejects, 20 show TS2367 or TS2845 in an unchanged TypeScript editor; no
other diagnostic appears.

## Compiler cost

Release, median of 8 runs in 3 alternating batches. Generated size and
the exact Rust heap peak are equal for shared accepted inputs.

| Shape | Source | Time baseline / prototype |
|---|---|---|
| dense comparisons | 16 KiB / 64 KiB | 7.22 / 7.19 ms; 28.99 / 28.93 ms |
| alias chain, 1 reference | 16 KiB / 64 KiB | 7.31 / 6.97 ms; 26.73 / 25.49 ms |
| alias chain, 8 references | 16 KiB / 64 KiB | 8.15 / 6.95 ms; 29.60 / 25.59 ms |

Growth gate: baseline 10.23 s (largest time ratio 6.49), prototype
9.97 s (6.63); largest output ratio 63.17 in both.

Checker size: the prototype changes only `checker/equality.rs` (292 to
249 lines; 47 removed, 4 added). It bypasses the alias bound and the
tested references and keeps the flow state; a full removal is larger.

## Not measured

The alternative of the proposal (a limited literal-type model that
keeps zero `tsc` diagnostics) has no prototype. Each of the 20
programs above with a diagnostic stays rejected under it.

## Decision

The owner chose type-based equality on 2026-10-12 (D17). Invariant 3
allows TS2367 and TS2845 at an accepted equality expression.

## Implementation record

- Codex task. `checker/equality.rs` 292 to 39 lines; the checker 2,312
  to 1,929 lines. Removed: tested sets, alias sets, literal edges,
  checkpoints and joins of the checker, the wide-operand attribute,
  `ALIAS_TEST_BOUND`. Kept, with their consumers: `bare_true` and
  `loop_break` (the end of a function, surface.md), declaration order
  and owners (C10, captures), `boolean_initializer` (the boolean `let`
  rule), loop depth (`break`, `continue`), type agreement (C1), and the
  emitter join writes (pipeline.md §5.10).
- Growth: the join and alias-chain shapes that measured only the
  equality flow state are removed; `nested-writes` and `spilled-and`
  stay for the emitter joins. 51 shapes; largest time ratio 6.60;
  largest output ratio 63.17; 9.75 s Debug.
- Random: 368 programs; 320 accepted, each equal to `node`, through the
  ABI, and `luau-analyze` clean; `tsc` gives TS2367 (83) and TS2845
  (55) in 93 programs, each at an accepted equality; no other code.
- `gate.sh full` passes: 77 entries in P2.3 or earlier.
