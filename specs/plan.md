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
- **P6 — performance and reload.** Workload measurements.

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
| D20 | A script names a host function with a module-level `declare function` in its own source. No ambient host declaration exists in the prelude | 2026-10-13 | `collisions.md` C18; owner decision at the P3 start |
| D21 | The host binds its capabilities at compile time: `ulua_compile` takes the grant list, and a declaration with no grant of the same name and signature is rejected. A grant can be revoked; a later call of it stops the run | 2026-10-13 | `abi.md` §6; owner decision at the P3 start |
| D22 | A host function error stops the run with `ULUA_HOST_ERROR`. A script cannot catch it (C17.7) | 2026-10-13 | `abi.md` §6; owner decision at the P3 start |

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
