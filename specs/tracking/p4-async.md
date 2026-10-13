# P4 async

Status: open. Decisions: D24 to D26 (`plan.md` §8).

## Slices

| Slice | Family | Entries |
|---|---|---|
| P4.1 | Async functions, `await`, promise values, the job queue | see "Exit checklist (P4.1)" |
| P4.2 | `Promise.all`, `then`, `catch`, `finally`, `Promise.resolve`, `Promise.reject` | later |
| P4.3 | Host async requests, completion ABI, pump | later |

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
