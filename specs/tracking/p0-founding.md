# P0 founding

Status: open.

## Exit checklist

- [x] `CLAUDE.md`, `specs/plan.md`, `specs/workflow.md`, `specs/rules-history.md`.
- [x] `specs/blocks/collisions.md` Rev 11, `specs/blocks/corpus.md`, `specs/blocks/surface.md` Rev 4.
- [x] `prelude/ulua.d.ts`, `tsconfig.json`, `package.json` (TypeScript 5.9.2).
- [x] Seed corpus: 24 accept, 68 reject, 3 trap.
- [x] Collision probes measured: `specs/tracking/p0-collisions.md`.
- [x] `tools/gate.sh quick|full`, `tools/hygiene.sh`.
- [x] The gate passes with the TypeScript from `npm install`.
- [x] Phase Review round 1: 0 CRITICAL, 6 MAJOR, 7 MINOR. All fixed (below).
- [x] Phase Review round 2: 4 MAJOR open or new (M4, M5, N1, N2, N3 as one class), MINOR items. All fixed (below).
- [x] Phase Review round 3: 6 MAJOR, 4 MINOR. All fixed (below).
- [x] Phase Review round 4: 4 MAJOR, 2 MINOR. All fixed (below).
- [x] Phase Review round 5: 1 MAJOR, 2 MINOR. All fixed (below).
- [x] Phase Review round 6: 1 MAJOR, 4 MINOR. All fixed (below).
- [x] Phase Review round 7: 1 MAJOR, 2 MINOR, 1 unpinned case. All fixed (below).
- [x] Phase Review round 8: 1 MAJOR, 3 MINOR. All fixed (below).
- [x] Phase Review round 9: 1 MAJOR, 3 MINOR. All fixed (below).
- [x] Phase Review round 10: 2 MAJOR, 5 MINOR. All fixed (below).
- [x] Phase Review round 11: 2 MAJOR, 5 MINOR. All fixed (below).
- [ ] Phase Review round 12 on the fixes.

## Measurements

- `sh tools/gate.sh full`: about 1 s wall time (TypeScript 5.9.2, node
  v24.18.0). The corpus witness runs two `tsc` programs and one node
  import per accept and trap entry (26), in one process. The hygiene scan reads each file once.
- Every reject entry is accepted by `tsc` (measured). The Ulua checker
  owns each rejection.

## Failures before the fix

- `a10` first compared two `const` booleans with `===`. `tsc` reported
  TS2367 (no overlap of literal types). The entry now takes parameters.
- `r14` first narrowed a `const` to `null`. `tsc` reported TS2339. The
  entry now takes a parameter.

## Phase Review round 1

| Id | Finding | Change |
|---|---|---|
| M1 | The position argument of `includes`, `startsWith`, `endsWith` counts UTF-16 units | C5.2 accepts one argument only; `r32` |
| M2 | `split` with a non-literal separator can be empty at run time | C5.2 requires a non-empty literal; `r33` |
| M3 | A lone surrogate has no UTF-8 form | C5.1 rejects it in literals; `r34` |
| M4 | `tsc` lets a string, array, or function into an object type | C1 rejects it; `r19` |
| M5 | No rule covered the temporal dead zone | C10 with two rules; `r20`, `r35` |
| M6 | `i + 1` rounds to an integer for some non-integer `i` | C6.1 checks `i` before the addition; `t03`; probe row |
| m7 | Count of agreeing number strings was 20, not 19 | corrected |
| m8 | The trim probe used different inputs; split and position cases were missing | probe fixed and extended |
| m9 | `"a" + n` uses Luau `..` formatting | C4.6 rejects `+` between a string and a non-string; `r21` |
| m10 | The witness missed config errors, unknown rules, reused numbers, non-diverging traps, empty output | each check added; each violating input fails (measured) |
| m11 | The hygiene scan missed temporary paths, home-relative paths, one-level climbs, the user name | `tools/hygiene.mjs`; each reviewer input fails (measured) |
| m12 | Rule and entry citations disagreed; rejected forms had no entry | C3, C4, C9 updated; `r22`–`r31`; the witness checks citations both ways |
| m13 | A wrong claim that node type stripping reads `erasableSyntaxOnly` | C8 corrected |

## Phase Review round 2

The class behind N2, N3, m9, and M3: `collisions.md` listed rejected
forms, so each unlisted form was open. The form changed: `surface.md`
is the closed list of accepted forms, and C11 rejects every other form.

| Id | Finding | Change |
|---|---|---|
| M5 | C10 missed a hoisted function used as a value, class bindings, and calls in nested blocks | C10 rule 2 covers every reference at any depth, and `class` bindings; `r36`, `r40`, `r41` |
| M4 | Type relations let a string reach an object type through arrays, unions, and results | C1: the checker owns assignability at every depth; array elements identical; `r43` |
| N1 | A write to the loop variable in the body was lost by a copy without copy-back | C9.2 copy-back on every path; C9.3 rejects a closure in the initializer; `a17`, `r42` |
| N2 | Template substitution and `String(x)` accepted every type | C4.8: `number`, `string`, `boolean` only; `r44` |
| N3 | Index signatures, library members, detached methods had no rule | `surface.md`, C6.8, C11; `r45`, `r46`, `r47` |
| M3 | Template literals and `\u{...}` escapes were not named | C5.1 names every escape form; `r37` |
| m9 | `+=` was not named | C4.6 names `+=`; `r38` |
| m10 | The witness did not check the `expected-error` id, the trap kind, or retired numbers | each check added; each violating input fails (measured) |
| m11 | Short temporary paths, the home variable, backslash climbs, and volume paths passed | patterns added; each input fails (measured) |
| m12' | corpus.md allowed contract sections in `rule` | corpus.md: collision ids only |
| N4 | String iteration had no entry; C6.7 did not close the member list | C5.3 and C6.7; `r39` |

## Phase Review round 3

| Id | Finding | Change |
|---|---|---|
| F1 | A Luau operator reads a local after its right operand; `n + inc()` gave 2, not 1 | C12 states the JavaScript evaluation order for the lowering; `a18` |
| F2 | A default on a `T \| null` parameter applies to `null` in Luau | C2 rejects it; `r48` |
| F3 | A closure in a `for` condition or update captured the head local | C9.3 rejects a closure anywhere in the head; `r49` |
| F4 | "After the declaration" did not say if the initializer is included | C10 measures from the end of the whole declaration; class bodies can refer to their class; `r50`, `r51` |
| F5 | A constructor read a field before its assignment | C13; `r52` |
| F6 | A class and an object type related structurally, with different call forms | C1.5 rejects the relation until P2; `r53`, `r54` |
| m1 | `push` result used as a value | surface.md: `push` is a statement; `r55` |
| m2 | No lowering requirement for empty needles | C5.2 states it; `a19` pins needle and separator edges |
| m3 | `a09` used an unannotated block arrow; grouping and field initializers were unlisted | `a09` annotated; surface.md lists grouping and field initializers |
| m4 | No trap kind for resource limits | CLAUDE.md invariant 5 and corpus.md §6: resource limit traps are outside the node comparison; entries at P1 |

## Phase Review round 4

The class behind M1, M2, and m1: C12 listed operators, so each
unlisted form was open. C12 now states one total rule: the lowering
never reads a binding, field, or element at a different point than
JavaScript. The listed items are instances, each pinned in `a18`.

| Id | Finding | Change |
|---|---|---|
| M1 | `xs[g()]` read `xs` after `g()` reassigned it | C12 total rule, instance 3; `a18` |
| M2 | The write range check was free to run before the right side grew the array | C6.3 and C12.4: the check runs at the store; `a18` |
| M3 | C10.3 covered one hop of hoisted calls | C10.3 is transitive; `r56` |
| M4 | Class members shared a table with metamethods and helpers | C14: no metatables; direct method calls; `a21` |
| m1 | The method receiver was read after the arguments | C12.2 names the receiver; `a18` |
| m2 | No rule kept the emitter from copying literal spelling that Luau reads differently | C14.3: literals from their values; `a20` |

## Phase Review round 5

| Id | Finding | Change |
|---|---|---|
| MAJOR 1 | Luau compiler limits (200 locals, 200 upvalues, 255 registers) failed accepted programs | C15: spill tables above 160 locals or upvalues; the depth limit bounds registers, measured in P1; `a22`, `a23` |
| MINOR 1 | `??` and `?:` had no stated lowering; `and`/`or` gives the wrong value for `false` | C2 and C3 state if-expressions; `a10`, `a14` pin a `false` operand |
| MINOR 2 | String literal emission did not exclude raw CR, NUL, or long brackets | C14.4: quoted strings with decimal escapes; `a20` pins CR and NUL |

## Phase Review round 6

| Id | Finding | Change |
|---|---|---|
| MAJOR A | C15 named three Luau limits; parameters, arguments, flat chains, statement nesting, closures per function, and upvalues across scopes still failed | C15 is one total rule: lowering forms plus compiler bounds with a limit diagnostic; P1 measures each bound and gates the input at and past it; any Luau compile error is a defect |
| MINOR B | The register budget of C15.1 and C15.3 was not stated together; temporaries were not spill candidates | C15.1 counts parameters, temporaries, and loop copies; C15.2 states the expression budget |
| MINOR C | Stale import count | corrected |
| MINOR D | Document lists differed | plan.md §6 lists `workflow.md`; README links `surface.md` |
| MINOR E | Non-STE modals | replaced |

## Phase Review round 7

| Id | Finding | Change |
|---|---|---|
| N1 | C15.1 fixed a spill form that needed one local per nested scope; 41 nested tables after 160 locals failed | C15.1 states the requirement; the P1 pipeline contract defines the form; combined gate inputs (locals with nested scopes, captures, depth) |
| N2 | C12, C14.3, C14.4, C15 cited Luau results with no probe | probes `evaluation-order`, `literals`, `compile-limit`; rows in `p0-collisions.md`; C15 states the non-constant condition |
| N3 | plan.md changed without a §9 record | plan Rev 1 with a §9 entry |
| — | A reassigned `for…of` binding: C6.6 did not say which length is live | C6.6: the array evaluated once; `a18` pins it |

## Phase Review round 8

The class behind MAJOR 1: surface.md listed some operators without
operand types. surface.md §4 now states the operand types of every
operator, and rejects any other operand type.

| Id | Finding | Change |
|---|---|---|
| MAJOR 1 | Unary `-` had no operand type; `-"abc"` is `NaN` in node and an error in Luau | surface.md §4 states operand types for every operator; C4.1; `r57` |
| MINOR 1 | BigInt literals were not excluded | surface.md and C4.1; `r58` |
| MINOR 2 | C15 limits and the 2^64 saturation had no probe | probe `luau-limits`; rows in `p0-collisions.md` |
| MINOR 3 | A trap golden with an empty prefix line passed when node printed nothing | the witness requires at least the prefix line count (measured) |

## Phase Review round 9

| Id | Finding | Change |
|---|---|---|
| MAJOR | Assignment `=` named no targets; `c.get = ...` replaced a method that C14.2 calls directly; `Infinity = 5` | surface.md §4 lists the assignment targets; C14.2 cites it; `r59`, `r60` |
| MINOR 1 | Local declarations were not in the statement list | surface.md §3 |
| MINOR 2 | Literal forms did not exclude spread, holes, shorthand, methods | surface.md §4; `r61` |
| MINOR 3 | Argument and `push` value flows relied on C1 implicitly | surface.md §4 and C1 name every value flow |

## Phase Review round 10

The class behind rounds 8 to 10: surface.md described forms in prose,
so each unstated sub-form (a binding pattern, a chain step, an operand
type) stayed open. surface.md Rev 3 is a table of syntax node kinds.
A node kind that the table does not list is rejected in every position.

| Id | Finding | Change |
|---|---|---|
| MAJOR A | A destructuring default applied to `null` in a parameter-default lowering | surface.md §1, §4: bindings and parameters are identifiers; `r62` |
| MAJOR B | `a?.f.g` did not short-circuit the rest of the chain | C2 and surface.md §3: one optional step, no step after it; `r63` |
| MINOR | Index operand type, assignment value use, rest and spread, unannotated initializer types | surface.md §1, §3, §4; `r64`, `r65`, `r66` |
| MINOR | C4.1 said `+` takes numbers only | C4.1 names both `+` forms |
| MINOR | A trap prefix compared print calls, not lines | the witness compares lines |

## Phase Review round 11

| Id | Finding | Change |
|---|---|---|
| MAJOR 1 | Narrowing was unspecified; `tsc` keeps a narrowed field across a call that clears it | C16: only `const` bindings and unassigned-by-closure locals narrow; never fields or elements; `r67`, `r68`; `a14` cites C16 |
| MAJOR 2 | A script binding `math` captured the `math.fmod` that C4.3 emits | C14.5: generated identifiers in a namespace apart from Luau globals; library reads through compiler-only names; `a24` |
| MINOR | `Utf8.byteLength` missing from the member row; conditional branch types; `void` arrow bodies; stale §4 references | surface.md Rev 4; collisions.md references |

## Gaps

- The Luau witnesses join at P1.
- The C15 bounds have no corpus entries until P1 measures their values.
- The hygiene scan of commit messages is manual.
- The node witness runs every entry in one process. A promise chain or
  a changed global can leak into a later entry (contrived).
- The live length of C6.6 has an entry for growth only, because no
  accepted member shrinks an array.
