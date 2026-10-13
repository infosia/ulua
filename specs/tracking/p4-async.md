# P4 async

Status: open. Decisions: D24 to D26 (`plan.md` §8).

## Slices

| Slice | Family | Entries |
|---|---|---|
| P4.1 | Async functions, `await`, promise values, the job queue | `a58` to `a61`; `r134` to `r141`; `t10` |
| P4.2 | `Promise.all`, `then`, `catch`, `finally`, `Promise.resolve`, `Promise.reject` | later |
| P4.3 | Host async requests, completion ABI, pump | later |

## Exit checklist (P4.1)

- [ ] collisions.md C19; surface.md Rev 24; pipeline.md §4.11;
  abi.md Rev 7 §4.7; corpus.md Rev 10 implemented.
- [ ] P4.1 entries pass in full; every earlier entry keeps passing.
- [ ] Direct tests: each C19 rejection; job order against `node` for
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
- [ ] Limits: nested immediate async starts at and past the resume
  depth give `limit:stack`; a drain inside `max_steps` and
  `max_memory_bytes`; growth shapes of await (sequential, nested, in
  loops, with `try`) inside pipeline.md §3.10.
- [ ] A random check of async programs against `node`, `tsc`, and
  `luau-analyze -j1`.
- [ ] Phase Review of the slice with no open CRITICAL or MAJOR finding.

## Contract pin (P4.1)

At `8e40e1f`, `tools/witness/luau.mjs` (`PHASE = "P4.1"`): `a58`,
`a59`, `t10` rejected `unsupported`; `a60`, `a61` C11; `r134` to
`r141` `unsupported`, want C19. Every earlier entry passes.

## Gaps

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
