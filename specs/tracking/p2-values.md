# P2 value representations

Status: open. Contracts: `pipeline.md` Rev 9, `abi.md` Rev 4,
`corpus.md` Rev 3, `surface.md` Rev 10, `collisions.md` Rev 22.
Decisions: D15, D16 (`plan.md` §8).

## Slices

Each slice ends with its contract, its corpus entries passing in full,
and `collisions.md` updated (`plan.md` §6). The `phase` header names the
slice; `tools/witness/luau.mjs` `PHASE` names the current slice.

| Slice | Family | Entries |
|---|---|---|
| P2.1 | Nested functions and arrows | `a23`, `a27`, `a30`; `r13`, `r16`, `r20`, `r35`, `r36`, `r41`, `r42`, `r49`, `r51`, `r66`, `r84` to `r88` |
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

Fix record: function types compare exactly; non-trailing defaults
give C11; an optional function type is in parentheses. Random: 600
programs, 432 accepted (each passes `tsc`, `node`, `luau-analyze`), 168
rejected (C1 96, C11 72). Gap: unions and arrays of function types are
not in the IR yet; P2.2 and P2.4 add the parentheses rule where they
print such types.

### P2.1 Phase Review round 2

Reviewer: fresh agent, diff `b4e42f3..26eee97`. 3 MAJOR, 3 MINOR.
The round 1 fixes close their classes (function type equality at every
value flow, defaults, type printing).

| Finding | Severity | Resolution |
|---|---|---|
| A closure created inside a hoisted declaration escapes C10: `const r = mk(); r(); let x = 1; function mk() { return () => x; }` prints 0; `node` throws ReferenceError | MAJOR | C10.3 Rev 22: references inside created closures belong to F; `r86` |
| Inferred function types have no bound: `() => () => ... a0` gives `internal` (output bound) at 1,060 bytes, a Luau "Exceeded allowed recursion depth" at 18.7 KB, and a `luau-analyze` SIGBUS at 14.5 KB | MAJOR | surface.md Rev 10: an arrow with a function result has a result annotation; pipeline.md §5.14.3 nested aliases; `r87` |
| A default resolves a name that the body declares: `f(a = g())` with a body `function g`; `tsc` TS2373 | MAJOR | surface.md Rev 10; `r88` |
| `s4_growth.rs` states 7.77 s; it measures 8.70 s | MINOR | S fix states the cost |
| C15 lists closures per function as a bound; §5.14.7 says none is needed | MINOR | C15 text |
| The tracking slice table, entry count, and checklist are stale | MINOR | This note |

Fix record: closure references propagate to the creating
declaration; arrows with function results need an annotation; defaults
cannot name body declarations; nested function types print through
their own aliases. Growth (Debug / Release time ratio, output ratio):
inferred chain 3.58 / 2.71 (C11); deep result types 3.72 / 3.54 (1.6);
deep parameter types 3.76 / 3.49 (0.58). Largest over all shapes 6.49
/ 6.43, output 62.7. Growth test 8.20 s Debug. The deep-type shapes
pass `luau-analyze` without the aliases, so they stay out of the
stress set (§5.11). Random: 640 programs, 425 accepted (each passes
`tsc`, `node`, `luau-analyze`), 215 rejected.

## Failures before the fix

## Gaps
