# Standard library — contract

Status: Rev 0, 2026-10-17 (P7 plan). Rev 0 states the design rules,
the absence rule D32, the measurement round, and the slice order. Each
slice adds its detailed section here before its implementation opens.
Evidence is in `specs/tracking/p7-stdlib.md`.

The library follows the standard library contract of the owner's prior
project: its member sets, its edge rules, and its decisions. A section
states each change from that contract with the Ulua rule that forces
it.

## 0. Design rules

1. **The `tsc` side is the ES2022 library.** `tsconfig.json` loads
   `lib: ["ES2022"]`. The prelude does not declare a global again. The
   prelude merges members into a library interface only to narrow a
   result type (as `pop(): T` does) or to add a member that the library
   lacks. The compiler accepts a closed subset of the library
   (collisions.md C11). It rejects every other member with a
   diagnostic that names the rule.
2. **Implementation forms, in order.** Each accepted member uses the
   first form that agrees with `node` on its measured input classes:
   1. a Luau builtin through a prelude alias (`FASTCALL`, pipeline.md
      §5.20);
   2. a fixed Luau expression around a builtin (as C4.5 formats a
      number);
   3. a prelude Luau helper that implements the ECMAScript algorithm;
   4. a native helper that the product registers in the VM. This form
      needs a measurement that shows that forms 1 to 3 cannot agree or
      cost more than the stated bound (for example Unicode case data).
3. **Values keep their representation (invariant 4).** A number, a
   string, a boolean, an array, and a record stay plain Luau values. A
   container that JavaScript defines with an order (`Map`, `Set`) is a
   prelude table that the script cannot read, as a promise is (D24).
4. **Determinism and capabilities (invariant 1).** Each accepted
   operation is deterministic for a given instance state. `Math.random`
   draws from a PRNG that the instance owns, with a fixed default seed
   and a C ABI call that sets the seed. A clock reaches a script only
   through a host grant.
5. **Limits (invariant 2).** Each helper loop has the interrupt points
   of pipeline.md §5.19, and each allocation is charged to
   `max_memory_bytes`. A parser (`JSON.parse`) has a nesting bound and
   reports a parse failure, never a native stack overflow.
6. **Agreement (invariant 5).** A member joins only with a measurement
   of `node` and of the Ulua lowering over its edge classes (NaN, `-0`,
   the infinities, the integer edges near 2^53, the empty and the
   non-ASCII string) and a random sample. Each difference is a fixed
   rewrite, a trap, a rejection, or a named divergence in
   `collisions.md`.
7. **Variadic forms.** The surface has no variadic parameters. A member
   with variadic arguments (`Math.max`, `Math.min`, `Math.hypot`) is
   accepted at the fixed arities that a slice names.

## 1. Absence of a result (D32)

Where `node` gives `undefined` for a missing result, Ulua gives `null`
(`nil`). This follows the owner's prior project.

1. A member with a missing result (`find`, `findLast`, `Map.get`) is
   accepted when its result type `T` does not include `null`. The
   checker types the result `T | null`. The `tsc` view stays the
   library type `T | undefined`.
2. The program tests the result with `x == null`, `x != null`, `??`, or
   `?.`. These forms give the same result in `node` for `undefined` and
   for `null`.
3. `x === null` and `x !== null` on such a result are a named
   divergence (collisions.md C2): `x === null` is `true` here and
   `false` under `node`. An entry with this form is
   `js-comparable: no C2`. Teaching material writes `== null`.
4. A result type `T` that includes `null` is rejected: a miss and a
   stored `null` are the same `nil`. The program uses `has` and a total
   accessor. The prelude adds `getOr(key, fallback)` to `Map`.
5. The source keeps no `undefined`: the token, the type, `x?: T`, and
   `void` as a value stay rejected (C2).
6. The P7.0 measurement M6 states the `tsc` view of each form before
   P7.2 opens: an annotation `const r: T | null = xs.find(f)` gives
   TS2322 (to measure), so such a result is held through an inferred
   type.

## 2. Measurement round P7.0

Prototypes only; each production change reverts. Each item records
its numbers in the tracking note and a decision here.

| Id | Question | Measurement |
|---|---|---|
| M1 | `Math`: which members agree with a Luau builtin | each member in `node` and on the pinned VM over the edge classes and 10^5 random inputs; form 1, 2, or 3 per member |
| M2 | Number formatting and parsing | `toFixed`, `toPrecision`, `toExponential`, `toString(radix)`, `parseInt`, `parseFloat`, `Number(s)` against `string.format`, `tonumber`, and a prelude helper; the cost of each helper |
| M3 | `Map` and `Set` representation | insertion order with deletion: a key array with an index table and holes, compacted; the cost of `set`, `get`, `delete`, `size`, and `for…of` against a plain table |
| M4 | `JSON` | `stringify` as a serializer that the emitter writes per type against one helper; `parse` with a declared target type; throughput, the nesting bound, and hostile input |
| M5 | Case mapping | `toUpperCase` and `toLowerCase`: `string.upper` (ASCII only) against a prelude table and a native helper; data size and cost |
| M6 | Absence (§1) | the `tsc` view and the `node` and Ulua outputs of each form of §1.2 and §1.3 on `find` and `Map.get` |

## 3. Slices

Each slice: a section of this file, corpus entries (Red at the
contract pin where the form is new), the implementation, the gates,
and a tracking entry. The phase ends with a Phase Review.

| Slice | Area | Depends on | Exit |
|---|---|---|---|
| P7.1 | `Math`: the members and constants of the prior contract, `random` with the instance PRNG and its ABI seed call | M1 | each member has its edge row in a corpus entry equal to `node`; `random` gives the pinned sequence for the default seed and for a set seed; the §5.19 and analyzer tests pass |
| P7.2 | Absence (§1): `find`, `findLast` | M6 | `== null`, `??` entries equal `node`; the `=== null` entry is `js-comparable: no C2`; a `T \| null` element type is rejected |
| P7.3 | `Number` statics, `parseInt`, `parseFloat`, `toFixed`, `toPrecision`, `toExponential`, `toString(radix)` | M2 | each form equals `node` over the M2 classes, or its divergence is named |
| P7.4 | `Map`, `Set`, `getOr`, `for…of` over both | M3, P7.2 | insertion order equals `node` with deletion; `get` follows §1; each operation cost is stated and measured |
| P7.5 | String additions: `toUpperCase`, `toLowerCase`, `String.fromCharCode`, `String.fromCodePoint` | M5 | each form equals `node` on ASCII and non-ASCII text, or its divergence is named |
| P7.6 | `JSON.stringify`, `JSON.parse` | M4 | output equals `node` for each accepted type; a parse failure is the stated channel; the nesting bound has a gate test at and past the bound |

Deferred, each with a later contract: `Date` (a clock grant),
`RegExp` (a native engine with a step budget), more Error classes and
the URI functions.

Non-goals: `Intl` and locale behaviour, `Symbol`, `Proxy`, `Reflect`,
`eval`, `Function`, `BigInt`, `console` (the language has `print`),
typed arrays, `WeakMap`, `WeakRef`.

## 4. Consequences outside this file

- `benchmarks.md` §4 states that Ulua has no `Math`. After P7.1 the
  pair rule is revised: a pair can use `Math.floor` and `Math.abs`
  against `math.floor` and `math.abs`.
- `surface.md` §6 lists the library; each slice updates it.
