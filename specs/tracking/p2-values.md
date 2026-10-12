# P2 value representations

Status: open; P2.1 to P2.4 COMPLETE 2026-10-12. Contracts: `pipeline.md` Rev 10, `abi.md` Rev 4,
`corpus.md` Rev 3, `surface.md` Rev 10, `collisions.md` Rev 22.
Decisions: D15, D16 (`plan.md` §8).

## Slices

Each slice ends with its contract, its corpus entries passing in full,
and `collisions.md` updated (`plan.md` §6). The `phase` header names the
slice; `tools/witness/luau.mjs` `PHASE` names the current slice.

| Slice | Family | Entries |
|---|---|---|
| P2.1 | Nested functions and arrows | `a23`, `a27`, `a30`; `r13`, `r16`, `r20`, `r35`, `r36`, `r41`, `r42`, `r49`, `r51`, `r66`, `r84` to `r88` |
| P2.2 | Arrays | `a09`, `a13`, `a17`, `a31`, `a34`, `t01`, `t02`, `t03`; `r23`, `r24`, `r55`, `r64`, `r89` |
| P2.3 | Array methods; type-based equality (D17) | `a32`, `a33`, `a35`, `a36`, `t04`, `t05`; `r90` to `r96`, `r98`, `r99`; `r24` becomes `r24-array-find` (P2.2) |
| P2.4 | Strings | `a11`, `a12`, `a19`, `a20`, `a24`, `a37`, `a38`, `a39`, `t06`; `r09`, `r10`, `r18`, `r34`, `r37`, `r57`; retired `r08`, `r21`, `r29` to `r33`, `r38`, `r39` |
| P2.5 | Nullable values and records | `a14`, `a15`, `a40`; `r03`, `r04`, `r06`, `r14`, `r19`, `r28`, `r43`, `r44`, `r45`, `r48`, `r61`, `r62`, `r63`, `r67`, `r68`, `r69`, `r100` to `r107`, `a42`, `a43`; retired `r05` |
| P2.6 | Classes | `a16`, `a18`, `a21`; `r40`, `r47`, `r52`, `r53`, `r54`, `r59` |
| P2.7 | Exceptions | contract and entries to write |

## Carry-over into P2

- P2.2 runs entries that execute C12.6 (`push` order); P2.5 runs entries
  that execute C16.4 (`plan.md` §9 Rev 4).
- P2.5 and P2.6: C7 rule 3 lists no member read and no object operand;
  `a21` needs a revision.
- D16: no accepted form and no library call gives `undefined`. P2.4
  states the consequence for `x === null` in C2.
- Invariant 5 audit candidates (`tracking/p1-skeleton.md`): each slice
  decides its own with the user.
- A phase that exposes the `buffer` library classifies Luau errors by
  source location, not by text.

## Exit checklist (P2.5)

- [ ] D19 and collisions.md C1.5, C2 (strict null tests), C7 (object
  identity), C16, surface.md Rev 16 (D18 for nullable references), and
  pipeline.md §5.18 implemented.
- [ ] P2.5 entries pass in full, the C16.4 carry-over included (`r68`,
  `r69` execute it); P2.6 entries stay `unsupported`.
- [ ] Direct tests: field reads and writes, nested records, records in
  arrays, null tests in every form, `??`, `?.`, narrowing at each join
  and its limits; each against `node`.
- [ ] Growth shapes for records and narrowing; `luau-analyze` passes.
- [ ] Phase Review of the slice with no open CRITICAL or MAJOR finding.

Source: the owner's prior project for null tests and narrowing (C17
shared locations, C24 indexed reads). Changes: records are structural
object types (D19, owner); `T | null` is accepted for every `T`
because `nil` represents it in Luau (invariant 4).

## Exit checklist (P2.4)

- [x] collisions.md C4.6 and C5 (byte semantics, the member list, the
  traps) and pipeline.md §5.17 implemented; the prelude declares `at`
  and `codePointAt` on `String`.
- [x] P2.4 entries pass in full; later slices stay `unsupported`.
- [x] Each member has a direct test: ASCII against `node`; non-ASCII
  against the byte rule; each trap.
- [x] Growth shapes for string members and concatenation;
  `luau-analyze` passes.
- [x] Phase Review of the slice with no open CRITICAL or MAJOR finding
  (round 2).

Source of the member set: the owner's prior project (byte measures).
Changes: case mapping and normalization stay rejected (no Unicode data
in Luau); no regular expressions; string ordering stays rejected.

## Exit checklist (P2.3)

- [x] collisions.md C6.9 and pipeline.md §5.16 implemented; the prelude
  declares `pop`, `shift`, `at` with result `T` and `findLastIndex`.
- [x] P2.3 entries pass in full; later slices stay `unsupported`.
- [x] Each method has a direct test against `node` for its position
  and edge rules; `sort` stability; the `array-modified` trap.
- [x] Growth shapes for method calls; `luau-analyze` passes.
- [x] §5.7 loop locals end with their loop, after the measurement it
  names; 200 sequential `for (let i...)` loops in one function compile.
- [x] D17 and D18 implemented with their corpus entries.
- [x] Phase Review of the slice with no open CRITICAL or MAJOR finding
  (round 4).

Source of the method set: the owner's prior project, with one change:
`find` and `findLast` stay rejected (D16).

## Exit checklist (P2.2)

- [x] `pipeline.md` §4.8 and §5.15 implemented.
- [x] P2.2 entries pass in full; later slices stay `unsupported`.
- [x] The array members of P2.2 are `length` and `push`; P2.3 adds the
  methods of C6.9 (the owner's prior project as the source).
- [x] Growth shapes for arrays (literals, nested arrays, many index
  operations, `for…of` over large arrays) and products with P2.1 shapes.
- [x] `luau-analyze` passes on the generated modules and the stress set.
- [x] Phase Review of the slice with no open CRITICAL or MAJOR finding
  (round 4).

## Exit checklist (P2.1)

- [x] `pipeline.md` §5.14 implemented; `UPVALUE_BUDGET` measured and
  pinned by a test at and past the bound.
- [x] P2.1 entries pass in full; later slices stay `unsupported`.
- [x] `luau-analyze` passes on the generated modules and the stress set.
- [x] Phase Review of the slice with no open CRITICAL or MAJOR finding
  (round 4).

## Measurements

### P2.3 Phase Review round 4

Reviewer: fresh agent, diff `9a74af7..64cce70`. No CRITICAL, no
MAJOR, 3 MINOR. No accepted probe gives a `tsc` code outside
invariant 3; the `never` exception matched only contradictory
equalities; D18 code matches its text; P2.3 methods and D17 hold.

| Finding | Severity | Resolution |
|---|---|---|
| The witness predicate rejects TS2845 when an operand is narrowed to `never` (`n === NaN` after `n === 1 && n === 2`) | MINOR | `corpus.mjs` counts a `never` operand as either type; corpus.md text |
| A member read on a reference narrowed to `never` gives TS2339 (`s.length` after `s === "a"` and `s === "b"`); unreachable while string members and records are `unsupported` | MINOR | Gap for P2.4 and P2.5 |
| The predicate has a second copy in `crates/ulua/tests/support/equality.mjs` | MINOR | Gap: the test imports the witness predicate (next implementation task) |

### P2.3 close

`sh tools/gate.sh full` passes: corpus 36 accept, 87 reject, 5 trap.
Running tasks at the close: none.

### P2.3 Phase Review round 3 (with D17 and D18)

Reviewer: fresh agent, diff `9a74af7..98079a6`. 2 MAJOR. D18 matches
its text; contextual types accept; P2.3 methods and D17 hold.

| Finding | Severity | Resolution |
|---|---|---|
| D18 omits element and member reads: `if (bs[i]) { const ys = [bs[i]]; ys.push(false); }` gives TS2345 and compiles | MAJOR | D18 reference forms include access paths; `r99` |
| A compound write to a reference that a contradictory accepted equality narrowed to `never` gives TS2322: `if (k === 1) { if (k === 2) { k *= 2; } }` | MAJOR | Owner decision: TS2322 and TS2345 that name `never` are inside invariant 3's exception; D18 narrows to boolean reference forms; `a36`; `r97` retired |

Information from the review: D18 as first written rejected `let best =
Infinity;`, `let acc = n;`, and array or function references; the
revision accepts number, string, array, and function references.

D18 revision record: only boolean reference forms (access paths
included) give C11. Random: 576 programs; 480 accepted, each equal to
`node` and `luau-analyze` clean; 96 rejected with C11. `tsc` codes on
accepted programs: TS2322 naming `never` 9, TS2345 naming `never` 2,
TS2367 10, TS2845 10, other 0. A member read `e.f` is checked by a unit
test; records arrive in P2.5.

### P2.3 Phase Review round 2 (with D17)

Reviewer: fresh agent, diff `9a74af7..963fea7`. 1 MAJOR, 2 MINOR. The
D17 type rule, C12 order, the kept flow parts, and the P2.3 round 1
fixes hold.

| Finding | Severity | Resolution |
|---|---|---|
| A narrowed reference copied into an unannotated `let` or array literal gives TS2322 or TS2345 at a later write: `if (s === "b") { let t = s; t = "c"; }`, `if (b === true) { const xs = [b]; xs.push(false); }`; the truth-test form `if (b) { ... }` existed before D17 | MAJOR | D18 (owner): a reference form at an inference position needs a type annotation (surface.md Rev 13) |
| `d17_equality.rs` states no cost (1.23 s) | MINOR | Cost statement |
| The witness predicate reads the narrowed `tsc` type; the compiler reads declared types (a false alarm is possible, no hole) | MINOR | Note; no case found |

D18 implementation record: the checker rejects a reference form at
the three inference positions with C11 at the offset of the form.
Random: 576 programs; 288 accepted (each equal to `node`, `luau-analyze`
clean; `tsc` TS2367 18 and TS2845 5, each at an accepted equality; no
other code), 288 rejected with C11. Growth and test fixtures that
copied references into unannotated `let` bindings now annotate them.

### P2.3 Phase Review round 1

Reviewer: fresh agent, diff `ebd372c..9a74af7`. 2 MAJOR, 5 MINOR.
Position arguments, search semantics, join, return values, traps, the
11 x 11 array-modified matrix, step and memory limits, `luau-analyze`,
§5.7 (400 sequential loops), and `tsc` hold.

| Finding | Severity | Resolution |
|---|---|---|
| C6.9 states the `reduce` callback `(acc, v, i)`; the code rejects it and accepts `(acc)` | MAJOR | Code follows C6.9; `(acc)` rejected |
| An inconsistent comparator orders differently from `node` (`[3, NaN, 1, 2].sort((a, b) => a - b)`); no divergence names it | MAJOR | C6.9 names the divergence with the measured cases; a one-parameter comparator is rejected |
| `map` and `flatMap` result element types have no C6.5 check (latent until null opens) | MINOR | C6.9 rule; code check |
| `p23_methods.rs` states no cost (3.07 s Debug) | MINOR | Cost statement |
| The helper selection falls back to a default body (`_ =>`) | MINOR | A total match |
| A prelude doc comment sits before the wrong declaration | MINOR | Fixed |
| C6.9 does not give the comparator parameters | MINOR | Fixed |

Fix record (P2.4 round 1): forward and backward KMP searches as Luau
loops; the review input ends with `limit:steps` in 65 ms at the default
limits. Library call audit: each call in the helpers does O(1) work
(`string.byte`, `string.sub` of at most 256 bytes, `utf8.codepoint`,
`math.*`, `typeof`, `tostring`, the fixed `gsub` of the format), or
work in proportion to its allocation (`table.insert` at the end,
`table.concat`). A window inside a code point is no match; helpers by
exact name; 14 stress modules (19.97 s analyzer). `p24_strings.rs`
6.76 s Debug; the large-input tests run under `measurements` (6.75 s).
2,889 cases equal `node`.

### P2.4 Phase Review round 2

Reviewer: fresh agent, diff `4ac7702..0c1ef44`. No CRITICAL, no MAJOR,
1 MINOR (this note lacked the gate line). 12 hostile run-time shapes
(KMP worst cases, `replaceAll("")`, trims and `for…of` over 8 MB,
`padStart(1e15)`, `repeat(1e12)`, self-concatenation) end with
`limit:steps` or `limit:memory` in at most 60 ms at the default limits
(about 60 ns per step). Search members: 9,000 ASCII cases equal `node`;
4,095 non-ASCII cases equal an independent byte-rule reference. The
reviewer stopped one line of inquiry (GC and runtime internals outside
the string helpers); see Gaps.

### P2.4 close

`sh tools/gate.sh full` passes: `cargo-test` 43 s, `luau-witnesses`
23 s (95 entries in P2.4 or earlier, 40 generated modules, 14 stress
modules); hygiene passes. Running tasks at the close: none.

### P2.4 Phase Review round 1

Reviewer: fresh agent, diff `4ac7702..c83c521`. 1 CRITICAL, 1 MAJOR, 6
MINOR. 152 ASCII edge cases equal `node`; non-ASCII results equal the
byte rule; no Luau pattern from script text; no `internal`.

| Finding | Severity | Resolution |
|---|---|---|
| `string.find` in plain mode is O(n*m) and holds one step: `"a".repeat(4194304).includes("a".repeat(2097152) + "b")` runs 104 s at the default limits | CRITICAL | pipeline.md Rev 18: searches are linear Luau loops (§5.17.2); a library call does O(1) work or work in proportion to its allocation (§6.3) |
| `startsWith`/`endsWith` trap when the comparison window ends inside a code point (`"café".endsWith("e")`) | MAJOR | C5.3: a window inside a code point is no match; `a39` |
| The trim helper and its dependencies enter every module that uses a string member (name matched by substring) | MINOR | Fix |
| 4 of 5 new stress shapes pass `luau-analyze` without the typed writes | MINOR | Remove them; keep `string-iteration` |
| `lastIndexOf` searches forward from 0 (`"a b ".repeat(60000).lastIndexOf(" ")` gives `limit:steps`) | MINOR | Backward search (§5.17.2) |
| `t06` states `js-comparable: yes`; `node` gives `""`, inside the static type | MINOR | `no C5` |
| C5.2 says `charCodeAt` gives 0 to 255; the code traps at continuation bytes | MINOR | C5.2 text |
| `p24_strings.rs` states no cost (6.9 s Debug) | MINOR | Cost statement |

### P2.5 Phase Review round 2

Reviewer: fresh agent, diff `917e951..b8cbf72`. 0 CRITICAL, 2 MAJOR, 1 MINOR. Each round 1 resolution holds.

| Finding | Severity | Resolution |
|---|---|---|
| A loop `break` or `continue` edge walks the path from the loop entry when a write precedes it: unit `if(y==null)break;y=g();` repeated, 16 KB to 64 KB, ratio 16.0 Debug and 14.8 Release (949 ms); the growth shapes put every fact before identical edges | MAJOR | C16.7 covers join edges; growth shape with a write between edges |
| D18 misses `o?.s ?? null` and `(o).s`, `(bs[0]).flag`: accepted, `tsc` gives TS2322 or TS2345 | MAJOR | surface.md Rev 18 reference definition, `r106`, `r107` |
| The witness `never` test matches a property named `never` | MINOR | the test matches `type 'never'` |

### P2.5 Phase Review round 1

Reviewer: fresh agent, diff `917e951..362fa4c`. 2 CRITICAL, 2 MAJOR, 5 MINOR.

| Finding | Severity | Resolution |
|---|---|---|
| C16 at the `for` update ignores `continue` edges: `for (...; ...; s += x.value) { if (x == null) { continue; } }` is accepted, Luau indexes `nil` (`internal`), `tsc` gives TS18047 | CRITICAL | C16.6, `r103` |
| `edge()` evaluates a `?:` test twice per level: 21 nested tests in 396 bytes take 3.46 s Debug; 58,356 bytes take 38.05 s Release | CRITICAL | C16.7; growth shape for nested tests |
| An object literal field without a contextual type is an inference position: `const o = { b: y }; o.b = null;` with `y` narrowed is accepted, `tsc` gives TS2322 | MAJOR | surface.md Rev 17 (D18 fourth position), `r104`, `r105` |
| TS2339 that names `never` after an assignment of `null`, no equality (`s = null; s?.length`); the witness accepts it | MAJOR | invariant 3 covers the `null` assignment; header `tsc: exception` (corpus.md Rev 6); `a42` |
| C16.1 omits `===` and `!==` | MINOR | C16.1 |
| `r68` executes C16.2, not C16.4; C16.4 for a `const` in an arrow has no entry | MINOR | `a43` |
| The "random" programs are fixed templates; the reject-code line is a constant | MINOR | wording here; the test counts the codes |
| `p25_values.rs` states no cost (1.07 s test, 2.3 s wall) | MINOR | the test states its cost |
| The C7 diagnostic text omits object types | MINOR | diagnostic text |

Fix (Codex): the `for` update joins the body end and each `continue`;
each condition node is computed once. The reviewer's 21-level `?:`
shape: 1,637.6 ms to 2.2 ms Debug (396 B); 31,037.8 ms to 8.0 ms
Release (58,356 B). Six new growth shapes, ratios 3.570 to 3.864;
growth test 7.234 s Debug, largest ratio 4.101. 13 D18 field forms
give C11 (`tsc`: TS2322 for 12, no error for 1). Template reject codes
counted: C1 40, C11 40, C16 120, C6 40. gate full: PASS, 123 entries.

### P2.5 implementation record

- Codex task, commit `e506cce`. Structural records with generated field
  keys; strict null tests; object identity; C16 as a flow fact
  (`checker/narrowing.rs`); canonical type identities.
- At the contract pin `2285494`, `a14`, `a15`, and `a40` gave
  `unsupported`.
- Direct tests: 81 accepted programs equal to `node`; 48 rejects. `tsc`
  gives TS2339 that names `never` in 2 programs, and no other code.
- Templates: 6 fixed program templates with a varied number, 480
  programs; 240 accepted, each equal to `node` and passing `tsc` and
  `luau-analyze`; 40 give TS2339 that names `never` after an assignment
  of `null` (no equality); 240 rejects from 6 fixed strings.
- Growth: 11 shapes; largest time ratio 3.936, largest output factor
  26.972 (record functions); Debug cost 5.978 s. Recursive types: ratio
  13.158 before canonical identities, 2.843 after.
- Reported gaps, closed by contract Rev 28 (`b4c9c3d`): a narrowed
  `const` read in a nested function declaration gives TS18047 (C16.4,
  `r101`); `{ __proto__: 1 }.__proto__` prints `[object Object]` under
  `node` and `1` in Luau (C1.6, `r102`).

### P2.4 implementation record

- Codex task. String members as prelude helpers; the shim traps
  `string-boundary` and `invalid-argument`; C4.6 concatenation.
- Direct tests: 2,781 accepted cases; 2,600 equal to `node` (ASCII), 94
  equal to the byte rule, 87 traps; each passes `tsc` and
  `luau-analyze`. Huge `repeat` and `pad*` end with `limit:steps` or
  `limit:memory` and recover.
- Random: 384 programs; 344 accepted (280 equal to `node`, 64 equal to
  the byte rule), 40 rejected.
- Growth: string members 3.24, member chains 3.22, concatenation 3.96
  (output 41.1), mixed concatenation 3.94, iteration 3.71; growth test
  9.86 s Debug.
- The stress set grows from 13 to 18 modules; the review checks the
  admission rule (§5.11) for the new ones.
- The equality test reads the witness predicate from
  `tools/witness/corpus.mjs` instead of a copy.

### P2.3 implementation record

- Codex task. Array methods of C6.9 as prelude helpers; `pop`,
  `shift`, `at` traps; `array-modified`.
- §5.7 measurement: the pinned Luau (O0 and O1) accepts 200 live locals
  and rejects 201; 1,000 ended blocks pass. 200 sequential `for` loops
  in one function compile and print 600, equal to `node`. The new
  boundary: 136 accepted, 137 rejected.
- Direct tests: 1,643 cases; 1,472 equal to `node`, 171 trap as the
  contract states; each accepted case passes `tsc` and `luau-analyze`.
- Random: 320 programs; 280 accepted (206 equal, 74 trap), 40 rejected.
- Growth: new shapes 3.59 to 3.91 per 4x, output 11.6 to 14.3; growth
  test 10.21 s Debug.
- Contract gap found: the index parameter of the `reduce` callback;
  C6.9 now states `(acc, v)` or `(acc, v, i)`.

### P2.2 implementation record

- Codex task. The shim trap function, checked array reads and writes,
  `length`, `push`, `for…of`, array types in the type table.
- Trap tests through the ABI: the output before the trap arrives; no
  poison; the next call works.
- Growth: 50 shapes; array shapes 3.83 to 4.02 per 4x (Debug); largest
  6.42 Debug, 6.38 Release; cost 7.45 s Debug, 2.37 s Release.
- Random: 550 programs; 444 run to completion, 26 trap as C6 states,
  80 rejected; each of the 470 accepted passes `tsc`, matches `node`
  (before a trap), and passes `luau-analyze`.
- `gate.sh full`: 73 entries in P2.2 or earlier; 23 generated modules.

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

### P2.1 Phase Review round 3

Reviewer: fresh agent, diff `b4e42f3..c9b7d0f`. 1 MAJOR, 3 MINOR.
C10.3 holds through each closure creation position; fuzz of 3,000
programs: 990 accepted, each passes `tsc`, `node`, `luau-analyze`; no
`internal`.

| Finding | Severity | Resolution |
|---|---|---|
| Each identifier read clones the whole type; each typed write hashes it. A wide nested function type read by many `k = g;`: Release 0.02 s at 16 KiB, 0.40 s at 64 KiB (ratio 20); peak RSS 23.8 MB and 425.9 MB | MAJOR | pipeline.md §3.10 Rev 10: interned types; product growth shapes |
| surface.md says `tsc` gives TS2373 for any body name; it does so only for a function declaration | MINOR | surface.md text |
| `FunctionTypes::add` recurses without a depth counter (§3.4); the parser bounds type nesting today | MINOR | Fix with the interned types |
| The function-result annotation rule tests a bare function type only; a later slice that adds arrays, nullable values, or records reopens the size class (`() => [() => 1]`) | MINOR | Gap below |

Fix record (round 3): a type table stores each type once; the IR
carries ids; `FunctionTypes::add` takes the depth counter. Product
shapes, 16 / 64 KiB, Debug / Release time ratio, output ratio, heap at
65,536 B: `k = g;` 3.88 / 3.70, 10.1, 9.8 MB; `const xi = g;` 3.77 /
3.67, 10.4, 12.0 MB; `q(g);` 3.86 / 3.66, 11.9, 11.8 MB; `k = k;` 3.77 /
3.73, 10.1, 9.6 MB (before: 343 to 426 MB). Worst heap: 180 bindings
and a 41-link `%` chain, 61.6 MB; `spilled-mod-39` 61.4 MB. Growth gate:
42 shapes, largest time ratio 6.36 Debug / 6.54 Release, output 62.7,
cost 8.18 s Debug. Random: 640 programs, 425 accepted, each passes
`tsc`, `node`, `luau-analyze`; no `internal`.
Codex reported that `luau-analyze` failed in its own shell with the
default stack and passed with `ulimit -s 65520`. The primary gate run
with `ulimit -s` 8,176 KB passes. Round 4 measured the stack need of
`luau-analyze`; see below.

### P2.1 Phase Review round 4

Reviewer: fresh agent, diff `b4e42f3..3d6200b`. No CRITICAL, no
MAJOR, 2 MINOR. About 90 probes and 1,200 random programs: each
accepted program passes `tsc`, `node`, `luau-analyze`; no `internal`.

| Finding | Severity | Resolution |
|---|---|---|
| §3.10 names a deep type; the product shapes use a wide type. A 45-level type read by many references measures 3.78 ms / 13.64 ms at 16 / 64 KiB (ratio 3.6), heap 10.3 MB | MINOR | §3.10 text names a wide type; the deep measurement is recorded here |
| The tracking header names `pipeline.md` Rev 9 | MINOR | Header fixed |

`luau-analyze` stack: the analyzer checks modules on up to 8 worker
threads with a fixed 512 KiB stack on macOS; `ulimit -s` does not
change it. Measured worker need: corpus and stress modules at most
32 KiB; the deepest accepted nesting modules at most 64 KiB. The main
thread needs 47 to 79 KB. The full run passes at `ulimit -s` 1024 and
8176, 4 runs each. The gate does not depend on the default stack.

### P2.1 close

`sh tools/gate.sh full` passes: 63 entries in P2.1 or earlier, 17
generated modules, 13 stress modules. Running tasks at the close: none.

### P2.2 Phase Review round 1

Reviewer: fresh agent, diff `4d8a9b9..182995b`. 2 MAJOR, 4 MINOR, 2
notes. Index checks, evaluation order, aliasing, closures in `for…of`,
traps, and budgets agree with `node` or trap as C6 states.

| Finding | Severity | Resolution |
|---|---|---|
| The output bound is reachable: each `[i]` (3 bytes) emits about 230 bytes; chained compound writes inside an arrow after 100 functions exceed 80 and give `internal`. The gate measured output at 32 KiB and had no chained-index shape | MAJOR | pipeline.md Rev 12: helper calls per access (§5.15), densest shape per construct at the ceiling (§3.10) |
| `a === [5, 2]` compiles; `tsc` gives TS2839 | MAJOR | C7 rule 2; `r89` |
| C6.3 says JavaScript makes a hole for every out-of-range write; for a negative, non-integer, NaN, infinite, or huge index it writes a property and keeps `length` | MINOR | C6.3 states the divergence with the measured cases |
| `pipeline.md` §7 states a prelude of 843 B; it is 921 B | MINOR | Updated after the fix |
| `p22_tests.rs:61-62` repeats an assertion; no test compiles a module-level literal at the budget | MINOR | Fix |
| C12.3, C12.4, C12.6 have no active entry in P2.2 (`a18` is P2.5) | MINOR | `a31` |
| Note: the prelude calls `getfenv()`, which turns off the Luau safe-environment optimizations | note | §4.8: the trap function is a chunk argument |
| Note: `for (const x of xs) if (x === 2)` gives C7 | note | C7 rule 3: a `for…of` binding is wide |

Fix record (P2.2 round 1): the trap function is a chunk argument;
element accesses call prelude helpers; `r89` gives C7; `a31` passes.
The growth test measures the densest shape of 97 constructs at 65,536
bytes: largest output ratio 63.14 (`%`), then `<` 59.9, `||` 61.3,
spilled `%` 62.7; the review reproductions now give 37.5 to 40.4.
Prelude 1,222 B. Heap over 121 shapes: 62.1 MB (templates), Rust
allocations 33.5 MB. Growth test 8.56 s Debug. Random: 640 programs;
512 accepted (448 equal to `node`, 64 trap per C6), 128 rejected with
C7; each accepted passes `tsc` and `luau-analyze`.

### P2.2 Phase Review round 2

Reviewer: fresh agent, diff `4d8a9b9..b783b31`. 1 MAJOR, 1 MINOR.
400 random array programs: 194 equal to `node`, 206 trap per C6 with
the `node` prefix; 0 differences; `tsc` and `luau-analyze` pass.

| Finding | Severity | Resolution |
|---|---|---|
| The densest-shape table is not the worst case: a fresh arrow per array literal pays one typed temporary declaration each; over spilled bindings with a 3-digit alias the ratio is 71.69 (4,699,351 B), above the recorded 63.14 | MAJOR | pipeline.md Rev 13: the output bound is a compiler limit, `limit:output` at the crossing statement; no margin claim |
| C6.7 says other members wait for the P2 array slice | MINOR | C6.7 text: a later revision with the user |

The class: a measured maximum is only a lower bound of the worst
case; each review found a denser shape (56.7, 62.7, 63.1, 71.7). The
bound becomes a limit diagnostic, so no accepted program reaches
`internal` through it.

Fix record (P2.2 round 2): `limit:output` at the crossing statement,
with offset, line, and column; `OUTPUT_FACTOR` stays 80. Largest
measured ratio 71.69 (`spillD`); `spillA` 69.15. Heap at 65,536 B for
`spillD`: 93.1 MB (sampled), Rust allocations 39.1 MB; this replaces
the 62.1 MB value of §7. Growth test 8.87 s Debug.

### P2.2 Phase Review round 3

Reviewer: fresh agent, diff `4d8a9b9..556ffc3`. 1 MAJOR, 1 MINOR.
`limit:output` is exact (68 programs at the bound and one byte below)
and points at the crossing statement in 9 positions; typical programs
have ratios 10 to 14; about 90 array probes agree with `node` outside
C6 traps; the heap matches §7.

| Finding | Severity | Resolution |
|---|---|---|
| An array literal of 129 elements (a 400-byte lookup table) gives `limit:temporaries`; the diagnostic points at the later `print` statement | MAJOR | pipeline.md Rev 15: constant elements in place, blocks of at most 32 temporaries (§5.15.2); limit diagnostics at the causing form (§6.0) |
| The final output check compares only the counter with the bound | MINOR | §3.10: the final check compares the counter with the text length |

Fix record (P2.2 round 3): constant elements in place; computed
elements appended one per block (within the bound of 32); a 129-element
and wider literal compiles. One position test per limit code (7 codes)
checks offset, line, and column. A counter and text-length mismatch is
`internal` (tested). Growth 16 to 64 KiB: constant literal 3.99 (output
20.3), computed literal 4.00 (44.3), nested 3.99 (43.6), module
computed 3.96 (40.3). Array method calls of C6.9 give `unsupported`
until P2.3. `gate.sh full`: 76 entries in P2.2 or earlier.

### P2.2 Phase Review round 4

Reviewer: fresh agent. No CRITICAL, no MAJOR, 2 MINOR. Literals of
31,000 constants, 12,000 calls, and 6,000 nested pairs compile and agree
with `node`; 1,500 index operations, 40-level index chains and nested
literals, and 20 nested `for…of` compile; limit positions hold.

| Finding | Severity | Resolution |
|---|---|---|
| §5.15.2 claims at most 32 temporaries per literal; each element keeps the temporaries of its own expression | MINOR | §5.15.2 text |
| `s4_growth.rs` states 8.87 s; it measures 9.83 to 9.94 s | MINOR | Comment states 9.94 s |

Note: 67 `for (let i = 0; ...)` loops in one function (3.3 KB) give
`limit:temporaries` (the P1 gap of sequential loop locals). Arrays make
this form common; the gap is proposed for an earlier slice than P6.

### P2.2 close

`sh tools/gate.sh full` passes: 76 entries in P2.2 or earlier, 25
generated modules, 13 stress modules. Running tasks at the close: none.

## Failures before the fix

## Gaps

- P5: a review of GC work and runtime internals outside the generated
  helpers against the run-time bound of §6.3 (a P2.4 reviewer did not
  finish this line).

- Closed in P2.5: TS2339 that names `never` joins invariant 3's
  exception; `a41` pins it.
- The witness predicate has a copy in `crates/ulua/tests/support/
  equality.mjs`; the next implementation task makes the test import
  `tools/witness/corpus.mjs`'s predicate.

- P2.2 and P2.5: an arrow whose inferred result contains a function
  type at any depth (array element, nullable member, record field)
  needs a result annotation, as a bare function result does now.
- P2.2 and P2.5: the parentheses rule for function types applies to
  every composite type they add (unions, arrays).
