# Ulua for TypeScript developers: examples

Each script shows one area where Ulua differs from TypeScript. Read
the comments next to the code: they say what you write in Ulua, why,
and what `node` prints where the output differs. The
general runner (`examples/run/host.c`) runs each script, and a gate
test compares its output with `expected-<name>.txt`. Every script
type-checks under `tsc --strict` with `prelude/ulua.d.ts`.

| Script | Shows |
|---|---|
| [`numbers.ts`](numbers.ts) | `number` only, `%`, the infinities, `Math.pow` in place of `**`, parsing with a radix |
| [`absence.ts`](absence.ts) | `null` in place of `undefined`, `T \| null`, a miss of `find` and `Map.get`, `getOr` |
| [`equality.ts`](equality.ts) | `==` is `===`, one operand type, identity of records, NaN, `-0` |
| [`types.ts`](types.ts) | Records, nominal classes, composition in place of `extends`, function values |
| [`strings.ts`](strings.ts) | UTF-8 byte lengths and positions, code points, `Utf8.byteLength` |
| [`arrays.ts`](arrays.ts) | Array methods, an append at `length`, a read outside the array that traps |
| [`errors.ts`](errors.ts) | The `Error` family, `catch` with `instanceof`, an uncaught exception |
| [`json.ts`](json.ts) | `JSON.parse` with a target type, dropped fields, a mismatch that throws |
| [`async.ts`](async.ts) | Async functions in the job order of `node`, `Promise.all`, no event loop |
| [`rejected.ts`](rejected.ts) | A program that `tsc` accepts and the Ulua compiler rejects, with its diagnostic |

Run one script after you build the runner as
[`../README.md`](../README.md) shows:

```sh
target/examples/run examples/typescript/strings.ts
```

The runner exits with status 0 at the end of `main`, 1 for a
rejection, and 2 for a stop (a trap or a limit). The
[TypeScript tutorial](../../docs/tutorial-typescript.md) describes each
difference, and
[`specs/blocks/collisions.md`](../../specs/blocks/collisions.md) states
each rule with the measured outputs of both sides.
