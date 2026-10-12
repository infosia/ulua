# P2 value representations

Status: open; P2.1 and P2.2 COMPLETE 2026-10-12. Contracts: `pipeline.md` Rev 10, `abi.md` Rev 4,
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
| P2.3 | Array methods | `a32`, `a33`, `t04`, `t05`; `r90` to `r94`; `r24` becomes `r24-array-find` (P2.2) |
| P2.4 | Strings | `a11`, `a12`, `a19`, `a20`, `a24`; `r08`, `r09`, `r10`, `r18`, `r21`, `r29` to `r34`, `r37`, `r38`, `r39`, `r57` |
| P2.5 | Nullable values and records | `a14`, `a15`; `r03` to `r06`, `r14`, `r19`, `r28`, `r43`, `r44`, `r45`, `r48`, `r61`, `r62`, `r63`, `r67`, `r68`, `r69` |
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

## Exit checklist (P2.3)

- [ ] collisions.md C6.9 and pipeline.md §5.16 implemented; the prelude
  declares `pop`, `shift`, `at` with result `T` and `findLastIndex`.
- [ ] P2.3 entries pass in full; later slices stay `unsupported`.
- [ ] Each method has a direct test against `node` for its position
  and edge rules; `sort` stability; the `array-modified` trap.
- [ ] Growth shapes for method calls (many calls, long arrays are a
  run cost, not a compile cost); `luau-analyze` passes.
- [ ] §5.7 loop locals end with their loop, after the measurement it
  names; 200 sequential `for (let i...)` loops in one function compile.
- [ ] Phase Review of the slice with no open CRITICAL or MAJOR finding.

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

- P2.2 and P2.5: an arrow whose inferred result contains a function
  type at any depth (array element, nullable member, record field)
  needs a result annotation, as a bare function result does now.
- P2.2 and P2.5: the parentheses rule for function types applies to
  every composite type they add (unions, arrays).
