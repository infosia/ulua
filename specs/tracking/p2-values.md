# P2 value representations

Status: open. Contracts: `pipeline.md` Rev 7, `abi.md` Rev 4,
`corpus.md` Rev 3, `surface.md` Rev 8, `collisions.md` Rev 19.
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

- [ ] `pipeline.md` §5.14 implemented; `UPVALUE_BUDGET` measured and
  pinned by a test at and past the bound.
- [ ] P2.1 entries pass in full; later slices stay `unsupported`.
- [ ] `luau-analyze` passes on the generated modules and the stress set.
- [ ] Phase Review of the slice with no open CRITICAL or MAJOR finding.

## Measurements

## Failures before the fix

## Gaps
