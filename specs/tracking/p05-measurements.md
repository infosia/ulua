# P0.5 measurements

Status: COMPLETE, 2026-10-11. M1 to M5 are measured and decided.

Every prototype of this round is outside the repository. No production
change lands.

## Exit checklist

- [x] M1 build root: numbers and a decision (D11).
- [x] M2 Rust and the Luau C API: numbers and a decision (D12).
- [x] M3 parser depth: numbers and a decision (D7, D13).
- [x] M4 number to string: numbers and a decision (D9).
- [x] M5 witness cost: numbers and a decision (D10).

## Setup

- `node` v24.18.0, TypeScript 5.9.2.
- Luau CLI and `luau-analyze` built in Release from the revision in
  `luau-revision.txt` (0.738), Unix Makefiles.

## M1 — build root

Two prototypes of one product: a Rust crate (staticlib and cdylib)
that exports `ulua_run`, compiles Luau source, and runs it in a
sandboxed state. A C host links the staticlib. macOS arm64, 8 cores,
Release, three runs each. Times in seconds.

- A: Cargo is the root. `build.rs` compiles Common, Ast, Bytecode,
  Compiler, and VM with the `cc` crate (62 objects).
- B: CMake is the root. Luau's own CMake targets build the Luau
  libraries, and a custom target runs `cargo build`.

| | A (Cargo, `cc`) | B (CMake drives Cargo) |
|---|---|---|
| cold build | 7.87 / 8.48 / 8.64 | 8.61 / 8.90 / 10.07, plus 1.64–1.70 configure |
| no-op build | 0.05 | 0.91–0.99 |
| touch one Luau VM `.cpp` | 6.74 / 6.85 / 6.97 (62 objects) | 1.29 / 1.35 / 1.36 (1 object) |
| touch one Rust file | 0.20–0.21 | 0.82–1.12 (with the host link) |
| staticlib | 20,032,904 B | 20,032,144 B |
| cdylib | 1,320,512 B | 1,320,528 B |
| stripped C host | 1,114,000 B | 1,130,560 B |

Flags of A: `-O3 -std=c++17 -DNDEBUG -fPIC -ffunction-sections
-fdata-sections`; the VM unit adds `-fno-math-errno` (as Luau's
CMakeLists). No `LUAI_*` define is necessary. The staticlib needs
`-lc++` on the host link line (`--print native-static-libs`).

Decision D11: Cargo is the build root, with `build.rs` and `cc`. A Rust
change is the frequent change: 0.2 s against 0.8–1.1 s. A Luau source
changes only at a revision pin; A then recompiles all 62 objects
(6.9 s). A needs no CMake. Sizes are equal.

## M2 — Rust and the Luau C API

Prototype: a C++ shim (`ulua_shim.cpp`, 286 lines, 15 of them a test
probe; header 67 lines). Every shim entry is `extern "C"` and
`noexcept`. Rust calls only shim entries.

Rule of the shim: every `lua_*` call that can raise runs inside
`lua_cpcall` or `lua_pcall`. A host function is a C++ trampoline. It
calls the Rust function, receives a status code, and raises
`luaL_error` after the Rust frame returns.

| Unwind mode | ns per successful call (1e6) | ns per failing call (1e5) | unprotected raise under `catch (...)` |
|---|---|---|---|
| C++ exceptions | 21.3–22.1 | 9,524–9,681 | caught, process continues |
| `LUA_USE_LONGJMP=1` | 22.4 | 210–222 | abort, exit 134 |

Cases, both modes, each followed by a script that runs:

| Case | Input | Result |
|---|---|---|
| runtime error | `error("x")`; arithmetic on nil | code 1 with the message |
| compile error | bad syntax; 5,000 nested parentheses | code 2; `Exceeded allowed recursion depth` |
| memory | `lua_Alloc` with an 8 MiB budget | code 3 `not enough memory`; recovery needs a full GC in the shim |
| time | interrupt callback with a budget; `while true`, also inside `pcall` | code 4; the error raises again after `pcall` |
| host function | Rust `print`; a Rust function that fails with 42; the same inside `pcall` | output; code 5; `pcall` returns `false` with the message |
| stack | Luau recursion; `__index` recursion; `pcall` recursion | code 1 `stack overflow`; `C stack overflow` |

Decision D12: build Luau with `LUA_USE_LONGJMP=1`. An error round trip
costs 43 times less, and a successful call costs the same. An
unprotected raise aborts the process: it never unwinds through a Rust
frame. The shim holds no object with a destructor between a raise and
its catch. `luau_compile` runs inside `catch (...)`, because it
allocates `std::string` (read from `lcode.cpp`, not run).

## M3 — parser depth

Parser: `oxc_parser` 0.146.0. STOCK is upstream
`0389c4010ef8298728f3e47fff8e2a9106d10045` (release oxc v0.146.0). The
fork https://github.com/infosia/oxc has two guard commits on top of it:

- `78f0be4010`: `ParseOptions::max_nesting_depth: Option<usize>`
  (default `None`) and `ParserReturn::depth_exceeded`. RAII guards on
  about 64 recursive cycles (expressions, statements, types, bindings,
  cover grammar). Two recursive AST walks become loops.
- `43ad6ed504`: with the guard on, no speculative arrow parse for a `(`
  head whose first identifier precedes `=`.

Harness: each probe parses one generated form on a thread with a fixed
stack, in a child process. A binary search finds the deepest level that
parses. B/lvl is the slope from 8 MiB to 64 MiB. macOS aarch64,
Release (opt-level 3, fat LTO) and Debug.

Selected STOCK rows (stack bytes per level; stack needed when the whole
source is one form):

| Form | Source B/lvl | Release B/lvl | Debug B/lvl | Release need at 64 KiB source | A bracket count sees it |
|---|---|---|---|---|---|
| `((x))` | 2 | 1,793 | 2,992 | 56 MiB | yes |
| `{a:{a:x}}` | 4 | 2,081 | 3,456 | 33 MiB | yes |
| `function(){return ...}` | 19 | 3,136 | 6,065 | 10 MiB | yes |
| `(a=(a=x))` | 4 | 2,704 | 5,953 | 42 MiB | yes |
| `a=a=x` | 2 | 912 | 752 | 28.5 MiB | no |
| `a=>a=>x` | 3 | 1,328 | 1,184 | 28 MiB | no |
| `a?a?x:b:b` | 4 | 1,088 | 672 | 17 MiB | no |
| `a**a**a` | 3 | 192 | 1,392 | 4 MiB (Debug 29 MiB) | no |
| `!x` | 1 | 128 | 224 | 8 MiB | no |
| else-if chain | 11 | 384 | 640 | 2.2 MiB | no |
| `A<A<T>>` | 3 | 1,568 | 3,425 | 33 MiB | no (`<` is ambiguous) |

The harness measured 30 forms. 17 of them nest
without brackets. `f()()()`, `a.b.c`, `a+a+a`, `T[][]`, and `A|B|C` do
not recurse in the parser.

- Quadratic arena: `(a=(a=x))` in STOCK uses 64·depth² bytes of arena.
  32 KB of source uses 4.1 GB and parses in 5.4 s. With the fork guard
  on: 1.0 MB and 2.8 ms.
- Fork head with the guard on: B/lvl as with the guard off. The worst
  B/lvl is 3,201 (Release) and 6,304 (Debug), function expressions. A
  guard of depth N needs about N × 6.3 KB of stack in Debug.
- After the parse: `needs_drop::<Program>()` is false. The default
  `oxc_ast_visit::Visit` walk recurses on the flat chains that the
  parser reads in a loop. It overflows a 1 MiB stack at 13,240 levels
  (Release, 80 B/lvl) and 1,885 levels (Debug, 560 B/lvl): 26 KB of
  source.

Decision D7 (confirmed): fork. A token pre-scan does not bound 17 forms
without a second parser for the recursive cycles. Without a guard, the
only bound is the source size, about 900 B of stack per source byte
(Release), and the `(a=` arena is quadratic.

Decision D13: every recursive walk over the AST in the compiler
(checker, IR, emitter) has a bound that the parser guard does not
give: a chain-length limit with a limit diagnostic, or an iterative
walk. P1 measures the stack per level of each walk with the guard.

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

- M2: `luau_compile` allocates outside the `lua_Alloc` budget. P1
  bounds it through the source size limit and measures it.
- M2: the time limit counts interrupt safepoints, not wall time.
- M2: the C stack limits are measured on an 8 MiB main thread only. P1
  measures them on the compiler thread and the run thread.
- M2: a Rust panic in a host function is not tested. P3 tests it.
- M1, M2: macOS arm64 only. P5 adds Linux and Windows.
- M3: `oxc_semantic` is not measured (crate `self_cell` is not in the
  local cache). Mixed-form nesting and the guard's reject path are not
  measured. P1 measures them with the fork pin.
- M5 measures hand-written Luau. P1 measures the generated corpus.
