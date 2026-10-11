# P2 value representations

Status: open. Contracts: `pipeline.md` Rev 8, `abi.md` Rev 4,
`corpus.md` Rev 3, `surface.md` Rev 9, `collisions.md` Rev 21.
Decisions: D15, D16 (`plan.md` §8).

## Slices

Each slice ends with its contract, its corpus entries passing in full,
and `collisions.md` updated (`plan.md` §6). The `phase` header names the
slice; `tools/witness/luau.mjs` `PHASE` names the current slice.

| Slice | Family | Entries |
|---|---|---|
| P2.1 | Nested functions and arrows | `a23`, `a27`; `r13`, `r16`, `r20`, `r35`, `r36`, `r41`, `r42`, `r49`, `r51`, `r66` |
| P2.2 | Arrays | `a09`, `a13`, `a17`, `t01`, `t02`, `t03`; `r23`, `r24`, `r55`, `r64` |
| P2.3 | Strings | `a11`, `a12`, `a19`, `a20`, `a24`; `r08`, `r09`, `r10`, `r18`, `r21`, `r29` to `r34`, `r37`, `r38`, `r39`, `r57` |
| P2.4 | Nullable values and records | `a14`, `a15`; `r03` to `r06`, `r14`, `r19`, `r28`, `r43`, `r44`, `r45`, `r48`, `r61`, `r62`, `r63`, `r67`, `r68`, `r69` |
| P2.5 | Classes | `a16`, `a18`, `a21`; `r40`, `r47`, `r52`, `r53`, `r54`, `r59` |
| P2.6 | Exceptions | contract and entries to write |

## Carry-over into P2

- P2.2 runs entries that execute C12.6 (`push` order); P2.4 runs entries
  that execute C16.4 (`plan.md` §9 Rev 4).
- P2.4 and P2.5: C7 rule 3 lists no member read and no object operand;
  `a21` needs a revision.
- D16: no accepted form and no library call gives `undefined`. P2.4
  states the consequence for `x === null` in C2.
- Invariant 5 audit candidates (`tracking/p1-skeleton.md`): each slice
  decides its own with the user.
- A phase that exposes the `buffer` library classifies Luau errors by
  source location, not by text.

## Exit checklist (P2.1)

- [x] `pipeline.md` §5.14 implemented; `UPVALUE_BUDGET` measured and
  pinned by a test at and past the bound.
- [x] P2.1 entries pass in full; later slices stay `unsupported`.
- [x] `luau-analyze` passes on the generated modules and the stress set.
- [ ] Phase Review of the slice with no open CRITICAL or MAJOR finding.

## Measurements

### P2.1 implementation record

- Codex task. C7 Rev 20 during the task: Codex measured that `tsc`
  keeps narrowing inside a callback (`if (p === 1) { take(() => p ===
  2); }` gives TS2367 for a parameter, a `let`, and a `const`); the
  handoff claimed the opposite.
- `UPVALUE_BUDGET` = 192: with the generated forms the pinned Luau
  accepts 200 upvalues and rejects 201. ABI tests at 192 and 193, a
  library capture, three nested levels with spills, per-iteration
  bindings.
- Growth, 8 to 32 KiB: nested functions 3.98 (output ratio 7.9),
  arrows 3.95 (27.4); largest over 35 shapes 6.37 (62.7). Growth test
  8.96 s Debug.
- Closures in `main` at 65,536 bytes: 8,187 (arrows), 1,830 (nested
  declarations); a 4-byte minimum gives at most 16,384 < 32,768.
- Random: 300 programs; 240 accepted, each passes `tsc`, matches
  `node` through the ABI, and passes `luau-analyze`; 60 rejected with
  the expected C7, C9, C10 codes.
- `gate.sh full`: 57 entries in P2.1 or earlier; 16 generated modules,
  13 stress modules.

### P2.1 Phase Review round 1

Reviewer: fresh agent, diff `b4e42f3..e7532c2`. 3 MAJOR. Fuzz: 400
closure and loop programs, 346 accepted, each equal to `node`, `tsc`
clean, `luau-analyze` clean; 400 deep-nesting programs, no
`internal`. Upvalue limits hold at two and three levels.

| Finding | Severity | Resolution |
|---|---|---|
| `?:` between function values takes the first branch type; `b ? g : f` with `g(a = 7)` and `f(a)` accepts `b()`; `tsc` TS2554; the run gives `internal` | MAJOR | C1.4 Rev 21: function types assign only when equal, defaults counted; `r84` |
| `function g(a = 1, b)` gets the type of `(a, b) => number`; `tsc` gives `a` the type `number \| undefined` | MAJOR | surface.md Rev 9: trailing defaults only; `r85` |
| A function-typed parameter with a default emits `(number, (number) -> boolean?) -> boolean`; `luau-analyze` reads an optional result | MAJOR | pipeline.md §5.14.3 parentheses; `a30` |

## Failures before the fix

## Gaps
