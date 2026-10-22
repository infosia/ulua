# Ulua — project plan

Rev 2 (2026-10-11; Rev 0 and Rev 1 the same day). This document records the founding design and the
phase plan. It is a working plan, not a contract. Corrections land in §9
with evidence. Read §9 before you reason from a claim here.

## 1. What is being built

An embedded scripting language for native host applications:

- **Surface syntax:** a TypeScript subset that stock `tsc` accepts with
  the Ulua prelude (`prelude/ulua.d.ts`). Editors get TypeScript tooling
  without change.
- **Compiler:** Rust. It parses with `oxc_parser`, checks the subset,
  builds a typed IR, and emits Luau source with `--!strict` annotations.
- **Runtime:** the Luau compiler and VM at the pinned revision
  (`luau-revision.txt`), with the Luau sandbox.
- **Embedding:** a C ABI with the prefix `ulua_`. The host owns the main
  loop, sets resource limits, grants capabilities, and calls exported
  script functions.

## 2. Prior art

| Project | Relevance | Difference |
|---|---|---|
| [TypeScriptToLua](https://typescripttolua.github.io/) | Translates TypeScript to Lua source | Keeps broad JavaScript semantics through a Lua helper library *(docs)* |
| [roblox-ts](https://roblox-ts.com/) | Translates TypeScript to Luau source for one host | Adds a runtime library and host-specific types *(docs)* |
| [AssemblyScript](https://www.assemblyscript.org/) | A TypeScript-syntax subset with ambient declarations gets editor tooling | Targets WebAssembly with its own runtime |

Ulua differs from the first two by invariant 4 and 5 in `CLAUDE.md`. It
rejects a form, lowers it to a fixed Luau expression that agrees, or
names the divergence. It adds no helper layer that imitates JavaScript
values.

## 3. Design invariants

The normative copies live in `CLAUDE.md`. In short: embedded with a C
ABI; hostile scripts; valid TypeScript subset; native Luau value
representation; JavaScript agreement by default, each divergence named; one
pipeline to typed Luau source.

## 4. Pipeline

```text
TypeScript source -> oxc_parser (forked, nesting guard) -> checker
  -> typed IR -> Luau source (--!strict) -> Luau.Compiler -> bytecode
  -> Luau.VM (sandboxed, safeenv)
```

- Rust owns parse, check, typed IR, and emission.
- Native code owns Luau compilation, loading, and execution.
- Compilation runs on a compiler-owned thread with a fixed stack.
- Every emitted identifier is generated. No user spelling becomes a
  Luau identifier.
- Emitted number constants do not depend on the process locale.

## 5. Witnesses

| Witness | What it checks | Gate |
|---|---|---|
| `tsc` (pinned in `package.json`) | acceptance; the reject header | quick |
| `node` | output of accept and trap entries with `js-comparable: yes` | quick |
| `luau-analyze --!strict` | the generated Luau | quick, from P1 |
| Luau VM through the C ABI | output of accept and trap entries | quick, from P1 |

## 6. Phase plan

- **P0 — founding.** `CLAUDE.md`, this plan, `specs/workflow.md`,
  `specs/rules-history.md`,
  `specs/blocks/collisions.md`, `specs/blocks/corpus.md`,
  `specs/blocks/surface.md`, the prelude,
  `tsconfig.json`, the seed corpus, `tools/gate.sh`, `tools/hygiene.sh`.
  Exit: the quick gate passes (`tsc` and `node` witnesses on every
  entry); each collision rule that states a measured difference has its
  Node and Luau results in `specs/tracking/p0-collisions.md`; hygiene is
  clean; Phase Review.
- **P0.5 — measurement round.** Prototypes only; every production change
  reverts. Items M1–M5 in §7. Exit: each item records numbers and a
  decision in §8.
- **P1 — vertical skeleton.** `number`, `boolean`, `let`/`const`,
  functions, `if`, `while`, `for`, templates with numbers. The minimal C
  ABI: context, compile, instance, call of an exported function with
  typed values, the output capability. Source, depth, time, and memory
  limits from the start. The measured compiler bounds of collisions.md
  C15, each with a gate test at and past the bound. The P0 carry-over
  items (§9 Rev 2). Exit: every P1 corpus entry matches its golden
  through the C ABI; `luau-analyze` reports zero errors; Phase Review.
- **P2 — value representations.** One slice per representation family:
  strings, arrays, nullable values, records, closures, classes,
  exceptions. Exit per slice: contract, corpus, collisions updated.
- **P3 — host interop.** Capabilities, revocation, host errors, C and
  C++ link tests.
- **P4 — async.** Coroutines, tasks, requests.
- **P5 — hardening.** Sanitizers, fuzzing, compiler memory bounds,
  Linux and Windows gates.
- **P6 — performance.** Workload measurements. The
  benchmark contract follows the owner's prior project: its timing
  method, its workload set, and its boundary-price measurement,
  changed where an Ulua rule forces it; the subjects are Luau and
  Ulua (D30). Contract:
  `specs/blocks/benchmarks.md`; slices in `specs/tracking/p6-performance.md`.
- **P7 — standard library.** The library of the owner's prior project,
  changed where an Ulua rule forces it: `Math`, absence of a result
  (D32), `Number` formatting and parsing, `Map` and `Set`, string case
  and code units, `JSON`. A measurement round P7.0 opens the phase.
  Contract: `specs/blocks/stdlib.md`; slices in
  `specs/tracking/p7-stdlib.md`. Exit: each slice of `stdlib.md` §3
  meets its exit; Phase Review.
- **P8 — developer command.** The `ulua` command: `check`, `run`,
  `link-flags`, and `build`, after the command of the owner's prior
  project, changed where an Ulua rule forces it (D35). Contract:
  `specs/blocks/cli.md`; evidence in `specs/tracking/p8-cli.md`. Exit:
  `cli.md` §7; Phase Review.

## 7. Measurement items (P0.5)

- **M1 — build root.** Cargo with a `build.rs` that compiles Luau versus
  CMake that drives Cargo. Measure cold build, one-file rebuild, and
  link size.
- **M2 — Rust and the Luau C API.** `lua_error` unwinds with a C++
  exception or `longjmp`. Neither can cross a Rust frame safely. Measure
  the smallest C++ shim that keeps every Luau error inside native code.
- **M3 — parser depth.** Stack use per nesting level of `oxc_parser`
  for each recursive form. Decide if the fork with a nesting guard is
  necessary.
- **M4 — number to string.** Compare `String(n)` in `node` with
  `tostring(n)` in Luau over special values and a random sample. Decide
  the lowering of a number in a template literal.
- **M5 — witness cost.** Time of `luau-analyze` and of the `node`
  witness per entry and per gate run.

## 8. Decisions

| Id | Decision | Date | Record |
|---|---|---|---|
| D1 | Embedded language with a C ABI; Luau VM in the product | 2026-10-11 | `CLAUDE.md` invariant 1 |
| D2 | Scripts are hostile | 2026-10-11 | `CLAUDE.md` invariant 2 |
| D3 | JavaScript agreement by default, in the native representation; each divergence named in `collisions.md` (revised by §9 Rev 3) | 2026-10-11 | `CLAUDE.md` invariants 4, 5 |
| D4 | Emit typed Luau (`--!strict`); `luau-analyze` is a witness | 2026-10-11 | `CLAUDE.md` invariant 6 |
| D5 | `T \| null` is the only absence; `undefined` is rejected | 2026-10-11 | `collisions.md` C2 |
| D6 | Strings are native UTF-8; encoding-dependent forms are rejected; byte operations use prelude names | 2026-10-11 | `collisions.md` C5 |
| D7 | Fork `oxc_parser` with a nesting guard; M3 confirms it. Pin: https://github.com/infosia/oxc branch `ulua`, commit `43ad6ed5043f524f78c16a2d874b5fc588942a15` | 2026-10-11 | §7 M3; `specs/tracking/p05-measurements.md` M3 |
| D8 | Name Ulua; C prefix `ulua_` | 2026-10-11 | this file |
| D9 | A number in a template or `String(n)` lowers to `tostring` with fixed rewrites of NaN, the infinities, zero, and the exponent | 2026-10-11 | `collisions.md` C4.5; `specs/tracking/p05-measurements.md` M4 |
| D10 | The gate runs `luau-analyze` and `node` once per gate run over all entries; lint warnings do not fail it | 2026-10-11 | `specs/tracking/p05-measurements.md` M5 |
| D11 | Cargo is the build root; `build.rs` compiles Luau with `cc` | 2026-10-11 | `specs/tracking/p05-measurements.md` M1 |
| D12 | Luau builds with `LUA_USE_LONGJMP=1`; Rust calls only `noexcept` shim entries; every raising call is protected | 2026-10-11 | `specs/tracking/p05-measurements.md` M2 |
| D13 | Every recursive AST walk in the compiler has its own bound: a chain-length limit or an iterative walk | 2026-10-11 | `specs/tracking/p05-measurements.md` M3 |
| D14 | License: MIT OR Apache-2.0, at the user's option | 2026-10-11 | `LICENSE-MIT`, `LICENSE-APACHE`, `README.md` |
| D15 | Where `tsc` acceptance depends on literal types or narrowing, the checker accepts a restrictive structural subset instead of a model of `tsc`. A later revision opens a form when a program needs it | 2026-10-11 | `collisions.md` C7 Rev 14; `tracking/p1-skeleton.md` round 4 |
| D16 | `undefined` is rejected. No accepted form and no library call gives `undefined` under `node`; a call whose miss is `undefined` under `node` is not in the library | 2026-10-12 | `collisions.md` C2; `tracking/p1-skeleton.md` |
| D17 | Equality acceptance is type-based (C7): one primitive type, or arrays with identical element types; no literal-type or narrowing rule. TS2367 and TS2845 at an accepted equality are outside invariant 3. D15 no longer covers equality | 2026-10-12 | `collisions.md` C7 Rev 25; `tracking/equality-measurements.md` |
| D18 | At the three positions where `tsc` infers a type (an unannotated `let` initializer, an array literal element without a contextual type, an arrow expression body without a result annotation), a boolean reference form (access paths included) needs a type annotation; a `never` narrowing after an accepted equality is inside invariant 3's exception (revised after the P2.3 round 3 review) | 2026-10-12 | surface.md Rev 13; owner decision after the P2.3 round 2 review |
| D19 | Records are structural object types (`type` aliases of object type literals, object literals); relation by identical field sets and types; classes are nominal and never relate to object types | 2026-10-12 | collisions.md C1.5 Rev 27; owner decision at the P2.5 start |
| D20 | The grant list is the only record of a host function. The library writes a `.d.ts` of `declare function` lines from a grant list; the host gives it to script authors and to `tsc`. A script declares no host function: every `declare` form is rejected (revised the same day; first form: a `declare function` in the script source) | 2026-10-13 | `collisions.md` C18, `abi.md` §6.3; owner decision at the P3 start, after a comparison with the owner's prior project |
| D21 | The host binds its capabilities at compile time: `ulua_compile` takes the grant list, and each grant is a host function name of the script with the grant's signature. A grant can be revoked; a later call of it stops the run | 2026-10-13 | `abi.md` §6.2; owner decision at the P3 start |
| D22 | A host function error stops the run with `ULUA_HOST_ERROR`. A script cannot catch it (C17.7) | 2026-10-13 | `abi.md` §6; owner decision at the P3 start |
| D23 | The C ABI has no binary compatibility rule and no experimental status: the project has one user. An ABI change updates `abi.md`, `include/ulua.h`, the implementation, and the tests in one commit | 2026-10-13 | `abi.md` Rev 6; owner decision at the P3 close |
| D24 | `Promise<T>` is a value: it can be stored, passed, returned, and awaited more than once. `async` functions, `await`, `Promise.all`, `then`, `catch`, and `finally` are in the surface. A promise is a prelude table whose state the script cannot read | 2026-10-13 | owner decision at the P4 start, after a comparison with the owner's prior projects (the one that holds promises as values) |
| D25 | Promise jobs run in the order of `node`: each reaction is one job of a FIFO queue, and the queue drains after each run that the host starts. An entry that interleaves tasks is `js-comparable` | 2026-10-13 | owner decision at the P4 start |
| D26 | An exception in an `async` body rejects its promise, and `await` throws it again: `try`/`catch` catches it (C17). A failed host request stops the run (D22) | 2026-10-13 | owner decision at the P4 start |
| D27 | A module exports zero or more functions; `main` is a corpus convention, not a language rule. A host calls any export | 2026-10-14 | `surface.md` Rev 26; owner decision while the examples were written |
| D28 | The Linux and Windows gates run on the owner's machines. The build and the gate scripts work on macOS, Linux, and Windows; the owner runs them and records the results | 2026-10-14 | `tracking/p5-hardening.md`; owner decision at the P5 start |
| D29 | Fuzzing is in-house: structured and byte-level mutation over the corpus and generated programs, time-boxed, under the sanitizer build. No coverage-guided tool (it needs a nightly toolchain) | 2026-10-14 | `tracking/p5-hardening.md`; owner decision at the P5 start |
| D30 | The benchmarks compare Ulua with hand-written Luau on the pinned VM only; Luau is 1.00×. No C, LuaJIT, or JavaScript subject | 2026-10-16 | `benchmarks.md` Rev 1; owner decision at the P6 start |
| D31 | Reload leaves P6; a later phase decides its meaning and contract | 2026-10-16 | `tracking/p6-performance.md`; owner decision |
| D32 | Where `node` gives `undefined` for a missing result, Ulua gives `null`, as the owner's prior project does: `find`, `findLast`, and `Map.get` are accepted for a result type without `null`; `== null` and `??` agree with `node`; `=== null` on such a result is a named divergence. The source keeps no `undefined`. Revises the library part of D16 | 2026-10-17 | `stdlib.md` §1; owner decision at the P7 plan |
| D33 | `Math` members whose Luau builtin differs from `node` or is absent use Rust `libm` 0.2.16 as a native helper: one result on every host OS; the 1- or 2-ulp difference from `node` is a named divergence | 2026-10-17 | `stdlib.md` §2, §5; collisions.md C4.9; owner decision after P7.0 M1 |
| D34 | An object literal whose type reaches `JSON.stringify` lists its fields in the declaration order of the type, so the output order equals `node` | 2026-10-17 | `stdlib.md` §2; owner decision after P7.0 M4 |
| D35 | A developer command `ulua` checks and runs a script through the product C ABI, prints the link flags, and builds a small host. `ulua emit` prints the generated Luau for a developer to read; no C ABI function returns it, and nothing accepts Luau (invariant 6). Revised the same day: the first form had no `emit` | 2026-10-20 | `cli.md` §2.5; owner request |
| D36 | The sanitizer build and `gate.sh sanitize` are for macOS and Linux only. Windows is outside them: MSVC has AddressSanitizer and no UndefinedBehaviorSanitizer. Narrows D28 for this gate | 2026-10-21 | `pipeline.md` §2 item 6 (Rev 71); owner decision |
| D37 | Linux x86_64 is a measured target, with macOS aarch64 and Windows x86_64. Each value that is the largest derivation over the measured targets covers Linux. A measurement tool that cannot measure a value on a target fails there | 2026-10-22 | `pipeline.md` §6 item 4, §7 item 1 (Rev 74); `benchmarks.md` §6 (Rev 6); `tracking/p5-hardening.md`; owner decision |

## 9. Corrections and revisions

Each entry records what changed, the evidence, and the consequence.

- **Rev 1 (2026-10-11).** §6 P0 lists `specs/workflow.md` and
  `specs/blocks/surface.md`. §6 P1 adds the measured compiler bounds of
  `collisions.md` C15 with gate tests at and past each bound. Evidence:
  P0 Phase Review rounds 2 and 6 (`specs/tracking/p0-founding.md`).
  Consequence: P1 exit includes the C15 gate tests.
- **Rev 2 (2026-10-11).** P0 closes after Phase Review round 12. The
  owner moved the review of the round-12 contract fixes (C12 instance 6,
  C16.4) and the round-12 MINOR items N3, N4, N5 to the P1 exit.
  Evidence: `specs/tracking/p0-founding.md` round 12. Consequence: the
  P1 Phase Review verifies C12.6 and C16.4, and P1 adds entries for N3,
  N4, and N5.
- **Rev 3 (2026-10-11).** Owner decision: invariant 5 changes from
  "accept only forms where JavaScript and Luau agree" to "agreement by
  default; a divergence is decided and named", the level of the
  owner's prior project. D3 follows. Evidence: `rules-history.md`,
  `tracking/p1-skeleton.md`. Consequence: `corpus.md` Rev 2 adds the
  `js-comparable` header; `collisions.md` Rev 16 states the divergence
  format; an audit of C1 to C16 found no rule that changes in P1.
- **Rev 4 (2026-10-12).** The P1 Phase Review verified C12.6 and C16.4
  by the contract text. P1 rejects their forms with `unsupported`, so
  no P1 program executes them. Consequence: the P2 exit runs entries
  that execute C12.6 and C16.4. Evidence: `tracking/p1-skeleton.md`
  round 9.
- **Rev 5 (2026-10-17).** §6 adds P7, the standard library. §8 adds
  D32: a missing library result is `null`. D16 stays for the source:
  no `undefined` token, type, or optional form. Evidence: the owner
  decision at the P7 plan; the planning probe in
  `tracking/p7-stdlib.md`. Consequence: `find`, `findLast`, and
  `Map.get` can join in P7.2 and P7.4; C2 names the `=== null`
  divergence when P7.2 lands.
- **Rev 6 (2026-10-17).** §8 adds D33 and D34 after the P7.0
  measurements. Evidence: `tracking/p7-stdlib.md` M1, M4. Consequence:
  P7.1 adds native helpers and a seed field to the C ABI; P7.6 adds a
  literal-order rule.
- **Rev 7 (2026-10-20).** §6 adds P8, the developer command. §8 adds
  D35. Evidence: the owner request; `cli.md` §6 lists each difference
  from the prior command and its reason. Consequence: the host flags
  of the gate test helper get one definition that the command shares.
- **Rev 8 (2026-10-20).** D35 adds `ulua emit` (owner request). Evidence:
  the corpus runner already writes the generated Luau for a developer
  (`pipeline.md` §8) with no C ABI accessor; `emit` keeps that line.
  Consequence: `cli.md` Rev 3 §2.5 and §7 criterion 8.
