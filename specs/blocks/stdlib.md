# Standard library — contract

Status: Rev 6, 2026-10-17. Rev 6: §7 rule 5 the format charge follows the work (P7.3). Rev 5: §7 rule 5 a fixed format charge (P7.3). Rev 4: §7 P7.3 `Number`. Rev 3: §6 P7.2 `find`, `findLast`, the absent type. Rev 2: §5 rule 5 `NATIVE_CALL_STEPS`; a70 row 27 reads `atan2(-0, -1)` (P7.1). Rev 1: the P7.0 decisions (§2) and the
P7.1 contract (§5). Rev 0 (P7 plan) states the design rules,
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

Decisions (P7.0, `tracking/p7-stdlib.md`):

- M1 (D33): the members whose builtin differs from `node` or is absent
  use Rust `libm` 0.2.16 as a native helper (form 4). The result does
  not depend on the host OS. The difference from `node` is a named
  divergence (collisions.md C4.9).
- M2: `toFixed` is form 2: `string.format` with fixed rewrites for an
  exact tie, `-0`, and a magnitude at or above 1e21 (P7.3).
- M3: `Map` and `Set` are the ordered prelude table with a separate
  NaN slot (P7.4).
- M4 (D34): an object literal whose type reaches `JSON.stringify`
  lists its fields in the declaration order of the type (P7.6).
- M5: Unicode default case conversion with special casing, form 4, at
  a pinned Unicode version, as the prior contract (P7.5).
- M6: §1 stands as written; `r === null` is the C2 divergence.

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

## 5. P7.1 `Math`

The member set of the prior contract, without its binary32 bit
accessors. Each member is a call of `Math.<name>(...)` or a read of
`Math.<CONST>`. `Math` as a value, a member read as a value, and every
other member are rejected (C11).

1. **Members and forms** (§0.2; M1 measured each row):

   | Form | Members |
   |---|---|
   | 1, builtin alias | `abs`, `ceil`, `floor`, `sqrt` |
   | 3, prelude helper | `round`, `sign`, `trunc`, `max`, `min`, `clz32`, `imul`, `fround` |
   | 4, native (Rust `libm`) | `acos`, `acosh`, `asin`, `asinh`, `atan`, `atanh`, `atan2`, `cbrt`, `cos`, `cosh`, `exp`, `expm1`, `hypot`, `log`, `log1p`, `log10`, `log2`, `pow`, `sin`, `sinh`, `tan`, `tanh` |
   | 4, native (instance PRNG) | `random` |

2. **Arity.** `atan2`, `hypot`, `pow`, `max`, `min`, and `imul` take
   exactly two arguments. Every other member except `random` takes
   one. `random` takes none. Another argument count is rejected
   (§0.7).
3. **ECMAScript edge rules**, each pinned by a corpus row: `round`
   rounds half toward +Infinity and gives `-0` for an argument in
   `[-0.5, -0]`; `sign` keeps `±0` and NaN; `max` and `min` give NaN
   when an argument is NaN, `max(+0, -0)` is `+0`, `min(+0, -0)` is
   `-0`; `trunc` keeps the sign of zero; `pow` gives NaN for a NaN
   exponent and for a base of `±1` with an infinite exponent, and `1`
   for an exponent of `±0`, also for a NaN base; `abs(-0)` is `+0`;
   `clz32(0)` is 32; `clz32` and `imul` convert as ECMAScript ToUint32
   and ToInt32 convert; `fround` rounds to binary32 to nearest even.
4. **Constants.** `E`, `LN2`, `LN10`, `LOG2E`, `LOG10E`, `PI`,
   `SQRT1_2`, `SQRT2` lower to number literals with the bits of the
   ECMAScript values. The emitter writes a literal that reads back to
   the same bits.
5. **Native helpers.** The product registers each form-4 member as a
   Luau C function before the sandbox freezes the environment. The
   helper calls the function of Rust `libm` 0.2.16
   (<https://crates.io/crates/libm/0.2.16>, pinned in `Cargo.lock`).
   `pow` adds the edge rules of rule 3 before the call. The prelude
   reaches each helper through a generated local; no script name
   reaches it. A helper call is a `CALL`, not a `FASTCALL`: it fires
   the interrupt of §5.19 and is never folded by the Luau compiler.
   The slice measures the cost per call and records it. Each helper
   call charges `NATIVE_CALL_STEPS` extra steps to `max_steps`, so the
   time per step stays inside the §5.19 ratio bound. Measured with no
   extra charge (P7.1 first form): `x = Math.sin(x)` in a loop gave the
   ratio 6.152 against the bound 6 (`Math.round`, a prelude helper:
   1.968); a native `sin` call cost 30.3 ns against 11.1 ns for the
   `floor` alias. The value is the smallest of 1, 2, 4, 8 whose
   largest Math-shape ratio is at most 5 (a margin of 1 against run
   noise); the tracking note records each measured ratio.
6. **`random`.** Each instance owns one xoshiro256++ generator, seeded
   by splitmix64 expansion of a 64-bit seed. The default seed is
   `0x5355425352454144`, as in the prior contract. The state words
   `s[0]` to `s[3]` are the first four splitmix64 outputs from the seed
   (`z = (x += 0x9E3779B97F4A7C15)`, `z = (z ^ (z >> 30)) *
   0xBF58476D1CE4E5B9`, `z = (z ^ (z >> 27)) * 0x94D049BB133111EB`,
   output `z ^ (z >> 31)`, all modulo 2^64). A draw takes the output
   `rotl(s[0] + s[3], 23) + s[0]` and then advances the state by the
   xoshiro256++ step. It maps the top 53 bits of the output to `[0, 1)`
   as `(x >> 11) * 2^-53`. The host
   sets the seed with `ulua_host.random_seed` before the module
   statements run, and with `ulua_instance_seed_random` later
   (`abi.md` §6, §7). The sequence does not depend on the host OS.
7. **Divergence (C4.9).** A form-4 member gives the value of Rust
   `libm`; `node` gives the value of V8. M1 measured 11 to 14,824
   differences of 1 or 2 ulp per 100,050 inputs (72,500 pairs), by
   member. `random` gives the sequence of rule 6; `node` gives an
   unseeded sequence. An entry that prints such a value is
   `js-comparable: no C4`. Its golden comes from a harness that calls
   Rust `libm` and the reference generator outside the product
   (principle 12).
8. **`**`** stays rejected (C4.4); `Math.pow` is the accepted spelling.
9. **Corpus.** `a69` (forms 1 and 3, the edge rules, the constants;
   `js-comparable: yes`), `a70` (form-4 members, `js-comparable: no
   C4`), `a71` (`random` at the default seed, `js-comparable: no C4`),
   `r149` (three arguments to `max`), `r150` (`Math.sin` as a value),
   `r151` (one argument to `hypot`). `r46` retires: `Math.round` is
   accepted; its row moves to `a69`.
10. **Exit.** The corpus above equals its goldens; `node` agrees on
    `a69`; a unit test per member compares the helper with the
    reference value at the edge classes; the seed tests of `abi.md`
    §8 pass; the §5.19 interrupt and analyzer tests and P6.3 pass; the
    cost per native call is recorded; `benchmarks.md` §4 states the
    revised pair rule.

## 6. P7.2 `find`, `findLast`, and the absent type

1. `xs.find(f)` and `xs.findLast(f)` on `T[]` follow the callback rules
   of collisions.md C6.9: `f` takes `(v)` or `(v, i)` and returns
   `boolean`; the array parameter is rejected. `find` scans from index
   0, `findLast` from the end. The prelude declares `findLast` (ES2023)
   with the result `T | undefined`, as the library declares `find`.
2. **The absent type.** The checker types the result `T?`: the element
   on a hit, `nil` on a miss. `T` excludes `null` (C6.5), so `nil`
   never is an element. `tsc` types the same value `T | undefined`.
3. **Accepted flows of `T?`**, each clean under `tsc` (M6):
   - the initializer of a `let` or `const` without a type annotation;
     the binding has type `T?`;
   - an assignment to a `T?` binding of a `T?` or a `T` value;
   - `x == null`, `x != null`, `x === null`, `x !== null`; they narrow
     as a null test of `T | null` narrows (C16);
   - `x ?? y`: the result is `T` when `y` has type `T`, and `T?` when
     `y` has type `T?`;
   - `x?.f` and `x?.m(args)` in the positions that C2 accepts for
     `T | null`.
4. **Rejected flows** (C2): a `T?` value to a declared type, a
   parameter, a result, a field, an array element, or `push`; an
   assignment of `null` to a `T?` binding; an equality with an operand
   other than `null`; a template substitution. Measured with `tsc`:
   `const r: number | null = xs.find(f)` gives TS2322, `r = null` on an
   inferred `r` gives TS2322, a `number | null` parameter gives TS2345,
   a `number | null` result gives TS2322; `r === 2` and `[r]` are clean
   under `tsc` and rejected here by C7 and C6.5.
5. D18 treats a reference of type `T?` as a reference of type
   `T | null`: it is a narrowable reference form.
6. **Divergence (C2).** On a miss, `x === null` is `true` here and
   `false` under `node`; `x !== null` is `false` here and `true` under
   `node`. `==`, `!=`, `??`, and `?.` agree. An entry with `===` or
   `!==` on a `T?` value is `js-comparable: no C2`.
7. **Corpus.** `a72` (hits, misses, `== null`, `!= null` narrowing,
   `??`, `?.` on a record; `js-comparable: yes`), `a73` (`=== null`
   and `!== null` on a miss; `js-comparable: no C2`), `r152` (a
   `T | null` annotation; `tsc` TS2322), `r153` (`null` assigned to a
   `T?` binding; TS2322), `r154` (a `T?` argument to a `T | null`
   parameter; TS2345), `r155` (a `T?` template substitution; `tsc`
   accepts). `r24` retires: its reason was D16.
8. **Exit.** The corpus equals its goldens; `node` agrees on `a72`;
   collisions.md C6.9 lists `find` and `findLast`; the D18 tests cover
   a `T?` reference.

## 7. P7.3 `Number` statics, parsing, and formatting

The member set of the prior contract (its §11), with the changes that
an Ulua rule forces.

1. **Constants.** `Number.MAX_SAFE_INTEGER`, `MIN_SAFE_INTEGER`,
   `EPSILON`, `MAX_VALUE`, `MIN_VALUE`, `POSITIVE_INFINITY`,
   `NEGATIVE_INFINITY`, `NaN` lower to literals with the ECMAScript
   bits, as the `Math` constants do (§5 rule 4).
2. **Predicates.** `Number.isNaN(x)`, `Number.isFinite(x)`,
   `Number.isInteger(x)`, `Number.isSafeInteger(x)` with `x: number`;
   an argument of another type is rejected (the library types the
   parameter `unknown`). The global `isNaN` and `isFinite` and
   `Number(x)` are rejected: they convert their argument.
3. **Parsing.** `parseInt(s, radix)` and `Number.parseInt(s, radix)`
   with the radix required; `parseFloat(s)` and `Number.parseFloat(s)`.
   ECMAScript semantics: leading StrWhiteSpaceChar skipped (the set of
   C5.2 `trim`), an optional sign, the longest valid prefix, NaN when no
   digit is read; `parseInt` with radix 16 skips a `0x` or `0X` prefix;
   `parseFloat` reads `Infinity` with a sign. The radix converts as
   ToInt32; a radix outside 2 to 36 traps `invalid-argument`
   (divergence: `node` gives NaN for 1 and 37 and reads 0 as the
   default). The result of `parseFloat` and of `parseInt` at radix 10
   is the correctly rounded double of the prefix.
4. **Formatting.** `x.toFixed(digits?)` (no argument is 0),
   `x.toExponential(digits?)` (no argument: as many digits as the
   value needs), `x.toPrecision(digits)` (required),
   `x.toString(radix)` (required). `digits` converts as
   ToIntegerOrInfinity; outside 0 to 100 (1 to 100 for `toPrecision`)
   it traps `invalid-argument`; a radix outside 2 to 36 traps
   `invalid-argument` (divergence: `node` throws `RangeError`, which a
   `catch` receives). NaN and the infinities give `NaN`, `Infinity`,
   `-Infinity`. `toFixed`: an exact tie picks the larger magnitude,
   `-0` gives no sign, a magnitude at or above 1e21 gives `String(x)`
   (M2). `toExponential` writes the exponent without padding
   (`0.00e+0`). `toString(10)` equals `String(x)` (C4.5).
   `toString(radix)` for another radix writes the fraction digits that
   `node` writes; the slice compares it with a table of `node` outputs
   and names each difference that remains.
5. **Implementation.** Each parse and format member is a native helper
   (form 4) in Rust, with no call of the host `printf` or `strtod`:
   their rounding and their exponent form depend on the host (M2; the
   prior contract §11.4). A helper call charges `NATIVE_CALL_STEPS`
   plus one step per `NATIVE_BYTES_PER_STEP` bytes of its string
   argument (measured: the smallest power of 2 that keeps a long-string
   shape at a §5.19 ratio of at most 5). A format member call charges
   `NATIVE_CALL_STEPS` plus `format_steps(x, digits, output)`, a closed
   formula over the length of the output and the binary exponent of
   `x`, stated in the code with its measurement. The formula follows
   the work: `toString(radix)` of a small fraction writes more than
   1,000 digits, and exact decimal rounding works over the whole
   exponent. Measured before the rule (P7.3): a fixed charge needed 512
   steps per call and still gave 7.162 in the timing gate for the
   longest binary fraction (`toFixed(100)` gave 30.6 at
   `NATIVE_CALL_STEPS` alone). The formula keeps every shape of a sweep
   over the value classes (subnormal, near 1, near 2^53, near
   `MAX_VALUE`), the digit counts (0, 20, 100), and the radixes (2, 3,
   10, 36) at a §5.19 ratio of at most 5; the tracking note records the
   sweep.
   Each result string is a Luau string charged to `max_memory_bytes`.
6. **Rejected** (C4): `Number(x)`, `new Number(x)`, the global `isNaN`
   and `isFinite`, `toLocaleString`, `x.toString()` without a radix,
   `toPrecision()` without an argument, a member read as a value.
7. **Corpus.** `a74` (constants and predicates), `a75` (formatting
   edges), `a76` (parsing edges), all `js-comparable: yes`; `t12`
   (`toFixed(101)` traps `invalid-argument`; `js-comparable: no C4`);
   `r156` (`Number(s)`), `r157` (the global `isNaN`), `r158`
   (`toString()` without a radix).
8. **Exit.** The corpus equals its goldens; `node` agrees on `a74` to
   `a76`; a unit table of at least 10^4 `node` outputs per member
   (random values, digit counts, radixes, and strings) equals the
   helpers, or each difference is named in collisions.md C4.10; the
   §5.19, analyzer, and P6.3 tests pass.
