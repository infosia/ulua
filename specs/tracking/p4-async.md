# P4 async

Status: open; P4.1 and P4.2 COMPLETE 2026-10-14. Decisions: D24 to D26 (`plan.md` §8).

## Slices

| Slice | Family | Entries |
|---|---|---|
| P4.1 | Async functions, `await`, promise values, the job queue | `a58` to `a61`; `r134` to `r141`; `t10` |
| P4.2 | `then`, `catch`, `finally`, `Promise.all`, `map` with a promise callback | `a62`, `a63`; `r143` to `r147`; `t11` |
| P4.3 | Async host requests, the completion ABI, the pump, call tasks | `a64`; `r148` |

## Exit checklist (P4.1)

- [x] collisions.md C19; surface.md Rev 24; pipeline.md §4.11;
  abi.md Rev 7 §4.7; corpus.md Rev 10 implemented.
- [x] P4.1 entries pass in full; every earlier entry keeps passing.
- [x] Direct tests: each C19 rejection; job order against `node` for
  the forms of Step 0 M4 that P4.1 accepts (await of a resolved and a
  pending promise, async return of a value and of a promise, nested
  async calls, interleaved tasks); await inside `try`, `catch`, and
  `finally` at nesting depths to the ceiling; each stop (each trap,
  `limit:memory`, `limit:steps`, `limit:stack`, each host stop)
  inside a resumed activation: the code it has without `async`, no
  `catch`, no `finally`, no later job, and the instance stays usable;
  an unhandled rejection, also one handled later in the same drain;
  async exports fulfilled, rejected, and pending; a drain that
  retains no completed job (memory after 10,000 serial awaits).
- [x] Limits: nested immediate async starts at and past the resume
  depth give `limit:stack`; a drain inside `max_steps` and
  `max_memory_bytes`; growth shapes of await (sequential, nested, in
  loops, with `try`) inside pipeline.md §3.10.
- [x] A random check of async programs against `node`, `tsc`, and
  `luau-analyze -j1`.
- [x] Phase Review of the slice with no open CRITICAL or MAJOR finding.

## Exit checklist (P4.2)

- [x] collisions.md C19.10 to C19.15 (C19.2 and C19.8 as revised);
  surface.md Rev 25 implemented.
- [x] P4.2 entries pass in full; every earlier entry keeps passing.
- [x] Direct tests: each C19.10 to C19.15 rejection; job order against
  `node` for `then` (value and promise results), `catch` (with and
  without a rejection), `finally` (with a fulfilled and a rejected
  source, and a callback that throws), `Promise.all` (settle order,
  first rejection, an empty array, later rejections handled), `map`
  with an async callback; an unhandled rejection of a derived promise;
  each stop inside a callback.
- [x] Limits and growth: chains of `then` and `Promise.all` over large
  arrays inside `max_steps` and `max_memory_bytes`; growth shapes
  inside pipeline.md §3.10.
- [x] A random check of P4.2 forms against `node`, `tsc`, and
  `luau-analyze -j1`.
- [x] Phase Review of the slice with no open CRITICAL or MAJOR finding.

Source: the owner's prior project that holds promises as values
(`then`, `catch`, `finally`, `Promise.all`; other statics rejected).
Changes: one-argument `then`; an `unknown` annotation on the `catch`
parameter; `Promise.all` without nullable or void elements (P4 Step 0
M4).

## Measurements (P4.2)

Implementation record (Codex), Debug, one machine:

- Direct tests: 81 rejections (0.033 s); 29 programs against `node`, `tsc`, `luau-analyze -j1`, and the ABI (0.682 s); 32 stops inside `then`, `catch`, `finally`, and `map` callbacks plus 8 controls (0.098 s): no `catch`, no `finally`, no later job, the instance stays usable.
- Random check (the P4.1 target, extended): 480 programs, 120 with P4.2 forms, 20 unhandled rejections, 0 differences (8.47 s).
- Growth, 9 shapes (`then` chains, adoption, arrows, `catch` and `finally` chains, `all` over literals, `map`, wide `then` and `all`): compiler 16 to 64 KiB at most 4.04; `luau-analyze` 4 to 16 KiB at most 5.16, ceiling at most 9.15 times `dense-assign`. Largest output 2,679,261 bytes at 64 KiB. 5,000 chained `then` and `all` over 5,000 elements run within 16 MiB. All 269 shapes: `measurements` 768 s.
- Decided gaps: an `unknown` parameter of a function value follows C17.4; a sync callback with a `Promise<void>` result is a C19.12 `finally` callback; an empty array follows the element type rule; rejected `all` inputs register in the order of C19.6.
- `cargo-test` 905 s in Codex's gate run.

### P4.2 close

Review rounds: 2 (round 2: 0 CRITICAL, 0 MAJOR, 1 MINOR, fixed). The round 1 test move had a timing regression, fixed with the `witnesses` lock (above). `sh tools/gate.sh full` by the primary agent: `cargo-test` 246 s, `luau-witnesses` 51 s (197 entries in P4.2 or earlier, 74 generated modules, 14 stress modules); hygiene passes. Running tasks at the close: none.

### P4.2 Phase Review round 1

Reviewer: fresh agent, diff `bbdcb7c..aed0365`. 0 CRITICAL, 2 MAJOR, 2 MINOR. Held: 31 order programs and 2,600 random programs (408 unhandled-rejection traps) equal `node` in output and message; `tsc` and `luau-analyze -j1` clean on the first 600; 15 derived-promise rejection cases; 81 checker probes; limits give `limit:memory` (1,000,000 chained `then`) and `limit:steps` (`all` over 2M elements); nesting gives `limit:nesting-depth`; growth: compile linear, analyzer 4x ratio at most 6.9, ceiling 7.7 times `dense-assign`.

| Finding | Severity | Resolution |
|---|---|---|
| `catch` and `finally` over a nullable source (`Promise<number \| null>`): Ulua and `node` agree, `luau-analyze -j1` gives a TypeError (`__ulua_new(nil :: (T)?)` infers `number` for `number?`); 16 of 24 accepted programs | MAJOR | Fix the typed lowering; direct tests over nullable sources |
| An unhandled rejection that module-level statements leave: Ulua stops `ulua_instance_new` with no output; the `node` runner writes `main` first; no rule named it | MAJOR | C19.7 names the divergence; `t11` (`js-comparable: no C19`) |
| `xs.map(async ...)` as an `ExpressionStatement` drops a `Promise<T>[]` | MINOR | C19.4: a statement whose type contains a promise type is rejected |
| The P4.2 pin cites `b17d7f2`, which the amend replaced | MINOR | Cite `3ea2b96` |

Fix measurements (Codex): the promise helpers keep the nullable type argument; 361 direct tests and 600 random programs (120 with nullable sources) pass `luau-analyze -j1` and equal `node`; `t11` gives no output and `trap:uncaught-exception`. A statement whose type contains a promise type gives C19.

Gate cost: the integration tests moved from 55 binaries to 6 (no name, input, assertion, or cost statement changed; the S3 allocator measurement keeps its own binary). After `cargo clean -p ulua`, `cargo test --quiet --workspace`: 415.1 s before, 241.5 s after (test run time 280.1 s to 208.7 s; build and link 135.1 s to 32.8 s). `cargo-test` in Codex's gate run: 226 s. `measurements` 887 s.

### P4.2 Phase Review round 2

Reviewer: fresh agent, diff `bbdcb7c..f6230dd`, priority `f6230dd`. 0 CRITICAL, 0 MAJOR, 1 MINOR. Held: the test move maps 55 old targets one to one to 6 binaries; 131 `#[test]` functions before, 133 after (2 new), no name lost; 8 feature gates before and after; the allocator measurement and the witness-time test keep their own binaries; scratch names stay unique within each binary; all 172 tests pass with `--features measurements` (858 s). Nullable helpers: 8 programs over 11 type forms and 5 edge forms equal `node` and pass `luau-analyze -j1`. C19.4: 41 statement forms. Unhandled rejection order through derived promises equals `node`. Nullable chains: 4 to 16 KiB ratio at most 3.0.

| Finding | Severity | Resolution |
|---|---|---|
| C19.4 counts method and function types as promise containment: `new Game();` with an `async` method and `factory();` returning `() => Promise<number>` give C19; `tsc` accepts and `node` prints `start`; `aed0365` accepted it | MINOR | C19.4: a function type and a method hold no promise value |

Fix (Codex): promise containment follows fields only; 10 rejections and 4 accepted controls; the random check unchanged (600 programs). A regression of the round 1 test move: the first gate run failed a compiler growth ratio (8.72, bound 8) that passes alone (3.99) and with `RUST_TEST_THREADS=1`. The merged binaries run the timing-ratio tests (`p25_growth`, `p3_growth`, `s4_growth`, and the others that assert a ratio) in parallel with other tests of the same binary, so the load changes the ratio (principle 15). Resolution: every test that asserts a time ratio moves into one binary whose tests take one process-wide lock; cargo runs test binaries one at a time.

Timing fix (Codex): 7 tests that assert a time ratio moved into the `witnesses` binary; each holds one process-wide mutex for its whole body; 133 test names, inputs, assertions, feature gates, and cost statements kept; the allocator measurement keeps its own binary. Three `gate.sh full` runs with the default thread count pass: `cargo-test` 242, 246, 244 s; largest ratios S4 6.40, 6.38, 6.32; P2.5 4.01, 3.99, 3.99; P3 3.93, 3.95, 3.92; P4 4.10, 4.54, 4.12; analyzer 8.50, 8.51, 8.54. `measurements`: 7 passed, 970.5 s, largest ceiling ratio 22.4.

## Exit checklist (P4.3)

- [ ] abi.md Rev 8 (§2 `ULUA_PENDING`, §5 `max_open_requests`, §6.2
  `start`, §6.3 async declarations, §6.4, §7); collisions.md C19.16;
  pipeline.md §6; corpus.md Rev 11 §7 (`hostLater` and the settle
  loop) implemented; `include/ulua.h` declares every new item.
- [ ] P4.3 entries pass in full; every earlier entry keeps passing.
- [ ] Direct tests (one per public function): start with each value
  type; complete with each type, a wrong type, an unknown id, twice,
  from a host callback (`ULUA_BUSY`); fail gives `host:request` and
  runs no `catch`; the pump order, `max_requests`, the step charge,
  `max_steps` per settle; a stop during a settle keeps the later
  completions recorded; `limit:requests` at and past the bound; call
  tasks for sync and async exports (pending, fulfilled, rejected,
  consumed); counts; revocation keeps open requests; instance free
  with open requests; the declarations text of an async capability;
  layout and link tests for the new items.
- [ ] Job order against `node` with the settle loop of corpus.md §7
  for interleaved requests, `Promise.all`, `then`, and `finally`.
- [ ] A random check of programs with async host requests against
  `node`, `tsc` (with the declarations), and `luau-analyze -j1`.
- [ ] Phase Review of the slice with no open CRITICAL or MAJOR finding.

Source: the owner's prior projects. The push completion that runs no
script and the host-driven step follow the project that holds promises
as values; the request cap and the stop of a failed request follow the
other. Changes: one pump run per settle (`node` macrotask order, Step 0
M6); call tasks for async exports.

## Measurements (P4.3)

Implementation record (Codex), Debug, one machine: 15 ABI tests (0.02 s); 14 order programs against `node` with the settle loop (0.54 s); the P4 random target extended to 720 programs, 120 with async host requests, 0 differences (11.3 s); 6 growth shapes: compiler 16 to 64 KiB at most 4.03, `luau-analyze` ceiling at most 2.08 times `dense-assign`, largest output 1,177,800 bytes; all shapes ceiling at most 22.1; `measurements` 1,079 s; `cargo-test` 276 s in Codex's gate run. Decided gaps, now in abi.md §6.4: a settle that stops counts in `*settled`; a completion copy that `max_memory_bytes` refuses gives `limit:memory` and records nothing; a stopped `ulua_call_start` does not write `*task`.

### P4.3 Phase Review round 1

Reviewer: fresh agent, diff `b105752..86432d3`. 0 CRITICAL, 1 MAJOR, 1 MINOR. Held: complete and fail validation in 13 forms; BUSY from `start` for 7 functions and from the output callback during a settle; pump order by record, `max_requests`; a stop inside a settle counts, keeps later completions, clears the queue, and runs no `catch` or `finally`; a failed request gives `host:request`; `max_steps` per settle; `limit:requests` at 256 and 257, freed by settles; call tasks for async and sync exports; revocation and frees keep open requests settleable; 0 leaks; ids unique per instance; declarations; 33 hand programs and 700 random programs equal `node` with the settle loop, `tsc` and `luau-analyze -j1` clean; 4 more shapes: analyzer at most 0.56 s at 64 KiB.

| Finding | Severity | Resolution |
|---|---|---|
| A `start` that returns non-zero leaves its request linked and open (`native/shim/requests.h:59-73`): 256 failed starts and every later async start gives `limit:requests`; with a large limit, 68,907 failed starts reach `limit:memory` and each later collection walks the leaked records | MAJOR | abi.md §6.4.1: a failed start discards its request (not open, no limit, invalid id); a test checks the counts |
| C19.6 names two host-started runs; pipeline.md §6 and abi.md §6.4.3 name four | MINOR | C19.6 lists all four |

## Contract pin (P4.3)

At the P4.3 contract commit, `tools/witness/luau.mjs` (`PHASE = "P4.3"`): `ulua-corpus` exits with "Unknown grant fixture: hostLater", so the Luau witnesses fail as a whole until the harness implements the fixture. `node tools/witness/corpus.mjs` passes (`tsc` and `node` for `a64` and `r148`).

## Contract pin (P4.2)

At `3ea2b96` (P4.1 compiler, as at `bbdcb7c`), `tools/witness/luau.mjs` (`PHASE = "P4.2"`): `a62`, `a63` rejected C19. `r143` to `r147` already give C19, since P4.1 rejects every promise member (C19.2 of P4.1); they pin the forms that stay rejected. Every earlier entry passes.

## Contract pin (P4.1)

At `8e40e1f`, `tools/witness/luau.mjs` (`PHASE = "P4.1"`): `a58`,
`a59`, `t10` rejected `unsupported`; `a60`, `a61` C11; `r134` to
`r141` `unsupported`, want C19. Every earlier entry passes.

## Measurements (P4.1)

Implementation record (Codex), Debug, one machine:

- Corpus: 188 entries pass; 71 generated and 14 stress modules pass `luau-analyze -j1`.
- Direct tests: 104 rejections (50 C19 forms, 54 `then` member forms); 74 programs against `node`, `tsc`, and `luau-analyze` (15 direct, 55 await nesting, 4 nested `then`). Await nesting ceilings: `try` 20, `catch` 16, `finally` 16.
- Stops in a resumed activation: 16 fixtures (traps, memory, steps, stack) and 3 host stops (`host:capability`, `host:revoked`, `host:result`): no `catch`, `finally`, or later job; the instance stays usable; the re-raise requests 0 allocations in 12 observed cases.
- Immediate async starts: depth 196 runs, 197 gives `limit:stack`.
- Drain: 0 jobs retained after 1,000 and 10,000 serial awaits (565,440 retained bytes in both).
- Random check: 240 programs (seed `0x41c1240026`), each through `node`, `tsc`, `luau-analyze -j1`, and the ABI; 0 differences; 10 unhandled rejections.
- Growth (`luau-analyze -j1`, minimum of 2 runs): 7 await shapes; 4 to 16 KiB largest ratio 4.86 (`try`); ceiling ratio largest 9.0 (`try`). All 194 shapes: 4 to 16 KiB largest 8.49, ceiling largest 31.2. `measurements` 600.7 s.
- `sh tools/gate.sh full` by the primary agent: `cargo-test` 584 s (P3 close: 473 s), `luau-witnesses` 53 s; hygiene passes.
- Decided gaps: a function value with a `Promise` result as an array method argument is rejected (C19.8 text widened to match); self-adoption rejects with `TypeError`, as in `node`.

### P4.1 close

Round 4 (fresh agent, diff `40f3806..31a661b`, priority `31a661b`): 0 CRITICAL, 0 MAJOR, 0 MINOR. Held: the 25 pending-by-final completion cases, sync and async, equal `node`; 500 random completion programs and 700 random async ordering programs equal `node` (73 unhandled-rejection messages exact); stops cross every protected form; recursion through `try` stops at depth 197 with `limit:stack`; upvalue boundaries with the helper (94+96 accepted, 95+96 rejected) and locals 120 to 300 equal `node`; nesting ceilings; at most 21 output bytes per source byte; 7 more protected shapes: 4 to 16 KiB at most 3.4, ceiling 4.7 times `dense-assign`. Review rounds: 4. `sh tools/gate.sh full` passes at `31a661b`: `cargo-test` 811 s, `luau-witnesses` 53 s (189 entries in P4.1 or earlier, 71 generated modules, 14 stress modules); hygiene passes. The close commit changes text only. Running tasks at the close: none.

### P4.1 Phase Review round 1

Reviewer: fresh agent, diff `40f3806..9d0179e`. 1 CRITICAL, 3 MAJOR, 2 MINOR. Held: 9 hand programs and 200 random programs equal `node` (interleaving, adoption, `try`/`finally` with pending awaits, loops, methods, captures, async calls in every expression position, recursion, self-adoption); unhandled rejection timing in 5 cases; 8 stop kinds in resumed and immediate activations (24 calls), no `catch` or `finally` after a stop; drain limits give `limit:steps` and `limit:memory`; deep immediate starts give `limit:stack`; 15 floating positions, 11 `then` forms, 9 relation forms, 6 identifier forms rejected; every accepted output passes `luau-analyze -j1`; await shapes 4 to 16 KiB ratio at most 3.9, ceiling 14 times `dense-assign`.

| Finding | Severity | Resolution |
|---|---|---|
| A nested async arrow tests an enclosing `let x: number \| null`, awaits, and reads `x + 1`; the enclosing function assigns `x = null` meanwhile: `tsc` and `node` accept (`1`); Ulua gives `internal` "attempt to perform arithmetic (add) on nil" and poisons the context | CRITICAL | C16.4: each `await` of a nested body ends the narrowings of enclosing `let` bindings and parameters |
| `await` in a parameter default of an async function or method is accepted; `tsc` TS2524, `node` SyntaxError | MAJOR | C19.3 names the position; fix the check |
| A script `class Promise` or `type Promise` is ignored by `Promise<T>`; `tsc` TS2315 | MAJOR | C19.2: a declaration named `Promise` is rejected |
| After `limit:memory` in a drain (async recursion), 3 of 50 runs leave every later call at `limit:memory` (300 calls of a sync export); the collection runs while the queue still holds the jobs | MAJOR | pipeline.md Rev 31 §4.11: a stop clears the queue before the collection |
| An async export that rejects with `A` while another promise rejects unhandled with `B`: Ulua reports `B`, the `node` runner `A` | MINOR | C19.7: the export's own rejection gives the message |
| `indexOf`, `includes`, `lastIndexOf` over `Promise<number>[]` compare promise identity; C19.2 rejects promise equality | MINOR | C19.2: array search over promise elements is rejected |

Fix measurements (Codex, Debug): 139 direct C16 rejections and 16 C19 rejections; 5 continuing and 3 ended narrowing cases; 360 accepted random programs (120 assign an enclosing binding while a nested async body is suspended) and 120 unsafe variants rejected, 0 differences against `node`, `tsc`, the ABI, and `luau-analyze -j1`; after a drain memory stop, 50 fresh instances run a sync export and a 1e6-push export. Await in `loop`, `catch`, and `finally` paths ends the narrowings through the existing write and join rules. Witness time: 4 to 16 KiB largest 8.41, ceiling largest 31.5; await shapes 4.94 and 9.13. `cargo-test` 685 s in Codex's gate run.

### P4.1 Phase Review round 2

Reviewer: fresh agent, diff `40f3806..3d8a747`, priority `9d0179e..3d8a747`. 0 CRITICAL, 1 MAJOR, 1 MINOR. The round 1 resolutions hold: 17 unsafe shapes of a narrowing read after a suspension and 26 join and path shapes are rejected with C16 (`node` reads `null` in each); 14 valid shapes are accepted and equal `node`; 700 random nested-async programs: 278 accepted with 0 differences, 422 rejected with C16; 6 stop kinds in a drain clear the queue, a later await of a promise that a stopped run left pending gives `async:pending`, and later calls succeed; memory recovers after a drain memory stop; the other round 1 fixes hold.

| Finding | Severity | Resolution |
|---|---|---|
| The await check of round 1 in `finalizer_jumps` walks the path from the current head to each recorded `break` head, once per enclosing loop and per `finally` with an `await`: `if(x==null)return 0;try{}finally{await t()}` repeated 1,440 times inside 16 loops with a `break` (64 KiB) compiles in 7.61 s Release, 52.3 s Debug at 8 loops; 4 times per doubling of the input; 0.06 s without the `break` | MAJOR | C16.7 holds (O(1) amortized per edge into a join); fix the check; growth shapes with `finally`, `await`, and loop jumps |
| Every binding named `Promise` gives C19 (`catch (Promise)`, a parameter `Promise`); C19.2 names a class and a type alias, and `tsc` and `node` accept the others | MINOR | Reject only a class or a type alias named `Promise` |

Fix measurements (Codex): the await check records the suspension as a write on the current edge, so the join rules carry it. Release D16/K1440 (64 KiB) 8.692 s before, 0.072 s after; compiler 16 to 64 KiB ratio at most 4.08 after. The 700 round 1 programs keep their results (278 accepted, 422 C16). Witness time: 200 shapes, 4 to 16 KiB largest 9.86, ceiling largest 38.2. `cargo-test` 710 s in Codex's gate run.

### P4.1 Phase Review round 3

Reviewer: fresh agent, diff `40f3806..b7ff68a`, priority `b7ff68a`. 0 CRITICAL, 1 MAJOR, 1 MINOR. Held: the round 2 check gives the verdicts and positions of round 1 on 22 hand shapes and 3,500 random programs (1,237 accepted, each equal to `node`, `luau-analyze` clean; 2,263 C16); 14 unsafe and 4 valid hand shapes; compiler time 16 to 64 KiB ratio 3.9 to 4.1 for 13 await, loop, and finalizer shapes (Release, 0.0315 s at 64 KiB); the round 2 regression shape 0.026 s (8.35 s at `3d8a747`); output at most 64.0 bytes per source byte; 13 `Promise` value bindings equal `node`; 16 await positions.

| Finding | Severity | Resolution |
|---|---|---|
| `luau-analyze -j1` time of `try` inside nested loops is past the §3.10 ceiling bound: 17 loops with `if(c)break;` around `try{}finally{await t()}` repeated to 64 KiB take 21.89 s, 70.6 times `dense-assign` (0.31 s); `try{r+=await t();}catch{}` 67.5 times; the sync `try{}finally{r=t()}` 131 times and 4 to 16 KiB ratio 14.9; time grows with the loop depth (5.67 s at 0 loops, 21.89 s at 17); output does not (3.85 MB); the product runs each in 0.1 s. The class dates from P2.7 | MAJOR | §3.10: change the lowering, or add a C15 bound; growth shapes with `try` under nested loops |
| An `await` in a `finally` also ends narrowings on loop jumps that do not pass it (4 programs that `tsc` and `node` accept give C16); the same at `3d8a747` | MINOR | C16.4 states the conservative rule |

Fix measurements (Codex): the cause was the branch-local writes of each protected call's completion locals and the type state that `luau-analyze` carries through them. Completion handling moves into one fixed helper; the caller tests the tag only when the source has a `return`, `break`, or `continue`. A type annotation or a loop-exit flag alone did not change the time. `luau-analyze -j1` at 64 KiB (Release, minimum of 2), before and after, at loop depth 0 and at the ceiling: async `finally` 5.62 to 1.04 s and 24.30 to 1.23 s; async `catch` 5.23 to 1.15 and 22.98 to 1.35; sync `finally` 7.28 to 0.99 and 43.72 to 1.18; sync `catch` 7.92 to 1.24 and 47.77 to 1.45; nested 5.65 to 0.84 and 13.86 to 0.98. 60 new growth shapes: 4 to 16 KiB largest 4.75, ceiling largest 4.85; all 260 shapes ceiling largest 22.4 (`dense-assign` 0.321 s). No C15 bound is needed. `measurements` 702 s; `cargo-test` 718 s in Codex's gate run.

## Gaps

- Gate cost: `cargo-test` grew from 473 s (P3 close) to 922 s (P4.2, `aed0365`). The 60 test targets run 296 s in sum (one target 138 s: the default witness-time test over 269 shapes); the other 626 s build and link 55 integration test binaries, each with the library and Luau. Resolved in P4.2 round 1: 6 binaries, `cargo-test` 226 s. Principle 15 asks for a stated reason per cost; review the P2 and P4.1 growth and random targets for overlap before P4 closes.

- Contract gap found in implementation: C19 allowed a `T` with a callable `then` member. Measured: Ulua returned the value, `node` and `tsc` resolved to the result of its `then`. C19.2 now rejects a `T` with a member named `then`.

## P4 Step 0 measurement

Measured at `40f3806` with prototypes; every production change reverted
(`git diff --exit-code -- crates/ native/ include/` exits 0). Pinned
Luau, `node` v24.18.0, Darwin arm64. The report and its prototypes
stay outside the repository.

- M1, suspension: `coroutine.yield` works inside the generated
  protected closures at every static nesting depth from 1 to 70 (also
  190 to 210), with break, continue, and return through
  `try`/`finally`, and inside `catch` and `finally`. The generated
  `forEach`, `map`, and merge-sort comparator helpers yield. A yield
  through a C boundary (`table.sort`, `lua_call` from a C trampoline,
  the host-call trampoline) gives "attempt to yield across
  metamethod/C-call boundary". An export that yields outside a
  dedicated coroutine gives `ULUA_INTERNAL` and poisons the context.
  Synchronously nested resumes reach "C stack overflow" at depth 198.
- M2, stops in a coroutine: `lua_resume` and `coroutine.resume` return
  the stop (trap, `limit:memory`, `limit:steps`, `limit:stack`,
  `host:capability`) as an error; a scheduler that discards it turns a
  memory or stack stop into success. With `coroutine.resume` and an
  immediate `error(v, 0)`, each stop reaches the ABI with its code and
  the instance stays usable; the re-raise requests no allocation.
  Direct `coroutine.wrap` adds a second location to a stack error: the
  ABI gave `ULUA_INTERNAL` and poisoned the context.
- M3, `luau-analyze -j1 --!strict`: a generic `Promise<T>` table type,
  typed await, and all over `{Promise<T>}` are clean with a type
  witness at the constructor (`fresh(nil :: T?)`) and a value
  assertion at the private resume boundary. The table type is
  invariant (`Promise<number>` does not flow to
  `Promise<number | string>`).
- M4, job order: a Luau prototype of the ES2022 job model for native
  promises (PromiseResolve, PerformPromiseThen, PromiseReactionJob,
  PromiseResolveThenableJob, Await, async start/return/throw, all,
  then/catch/finally per 27.2.5.3) against `node`: 40 hand forms and
  2,000 random programs (seed 240026), 2,040 lines, 0 differences,
  also through the public ABI. `Promise.all([Promise.resolve(null)])`
  and `Promise.all([(async () => {})()])` give one element in `node`
  and zero in a plain Luau array.
- M5, costs (product build, public ABI, medians of 7): 10,000 awaits
  of a resolved promise 3.57 ms, 22.3 charged steps each; of a
  non-promise value 5.89 ms, 30.2 steps each; 10,000 coroutine
  create and first resume 1.16 ms; one queued reaction 370 bytes
  retained. A queue that keeps consumed entries retains 5.6 MB at
  10,000 serial awaits. `luau-analyze -j1` of await-heavy output:
  0.0134 s at 4 KiB, 0.0345 s at 16 KiB (ratio 2.58).
- M6, host completions: a host fixture that settles one request per
  macrotask (`setImmediate` between) and the job model with one
  host-started run plus a drain per settle: 5 programs, 14
  completions, 0 differences.
