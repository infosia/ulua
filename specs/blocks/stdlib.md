# Standard library — contract

Status: Rev 12, 2026-10-18. Rev 12: §8 rule 1a number keys outside the `int` range (P7.4 defect). Rev 11: §8 rule 6 `await` in a traversal, §10 rules 5 and 6 two divergences (P7 Phase Review). Rev 10: §10 P7.6 `JSON`. Rev 9: §9 P7.5 string case and code points. Rev 8: §8 rule 6 an overwrite during a traversal does not trap (P7.4 review). Rev 7: §8 P7.4 `Map`, `Set`. Rev 6: §7 rule 5 the format charge follows the work (P7.3). Rev 5: §7 rule 5 a fixed format charge (P7.3). Rev 4: §7 P7.3 `Number`. Rev 3: §6 P7.2 `find`, `findLast`, the absent type. Rev 2: §5 rule 5 `NATIVE_CALL_STEPS`; a70 row 27 reads `atan2(-0, -1)` (P7.1). Rev 1: the P7.0 decisions (§2) and the
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

## 8. P7.4 `Map` and `Set`

The member set of the prior contract (its §10), with the changes that
an Ulua rule forces. Measured with `node` v24.18.0 (P7.4 contract).

1. **Representation (M3).** A `Map<K, V>` or a `Set<K>` is a prelude
   table that the script cannot read (§0.3): an index table from key
   to slot, a key array and a value array in insertion order, a
   separate slot for the NaN key, a live count, and holes that a
   compaction removes when the live count falls to half of the slots.
   Measured cost against a plain table (M3): `set` 1.3 to 1.5 times,
   `get` 1.2 to 1.3, `delete` 2.4 to 2.8 (with compaction), iteration
   1.5.
1a. **Number keys of the index table.** The index table never gets a
    number key `k` outside `-2^31 <= k < 2^31`. Such a key (an
    infinity, `1e300`, `2^31`, `-2^31 - 1`, `2^31 + 0.5`) is stored in
    the index table as the 8-byte string `string.pack("<d", k)`. A
    `Map<number, V>` or `Set<number>` has only number keys, so this
    string collides with no other key of the same table. The key array
    keeps the number itself (invariant 4), so iteration and `keys()`
    are unchanged. The slot counter stays below `2^31`: when it
    reaches `2^31 - 1`, `set` compacts before it adds a slot (a delete
    that traps `map-modified` skips its compaction). Reason: the
    pinned VM converts a number key with
    `int(d)` in the `GETTABLE` and `SETTABLE` fast paths and in
    `luaH_get` (`luai_num2int`, `VM/src/lnumutils.h`); the conversion
    is undefined behaviour for NaN, an infinity, or a value outside
    the `int` range (pipeline §5.20). Measured (P6.S7):
    `gate.sh sanitize` gave UBSan "is outside the range of
    representable values of type 'int'" at `lvmexecute.cpp:779` in
    `p74_tables::node_differential_ten_thousand_sequences` for the key
    `-inf`, at 70479aa and after P6.S7. The same class covers every
    other prelude table access with a key that a script supplies
    (pipeline §5.20); the implementation lists each such access.
2. **Key types.** `number` (SameValueZero: NaN is one key, `-0` is
   stored as `+0`), `string` (content), `boolean`, and a class, record,
   or array type (identity, as `node` compares object keys). A
   function type, a `T | null` type, a `T?` type, and a promise type
   are rejected as `K` (C7 does not define their equality). `V` has no
   restriction except §1.4 for `get`.
3. **Order.** Iteration follows the first insertion: `set` on a
   present key keeps its position; `delete` and a later `set` append
   it. Measured: `set a, set b, set a, delete b, set b, set c` gives
   keys `a,b,c` and values `9,3,4`.
4. **`Map<K, V>` API.** `new Map<K, V>()` (the type arguments come
   from the annotation or the call), `new Map<K, V>(other)` with a
   `Map<K, V>` source (a shallow copy in insertion order), `size`
   (read), `get(k)` (§1: the absent type `V?`; rejected when `V`
   includes `null`), `getOr(k, fallback)` (a prelude addition, total
   for every `V`), `set(k, v)` (returns the receiver), `has(k)`,
   `delete(k)` (returns whether an entry was removed), `clear()`,
   `forEach(f)` with `f` of `(v)` or `(v, k)`, and `for (const k of
   m.keys())` and `for (const v of m.values())` as the direct subject
   of `for…of`.
5. **`Set<K>` API.** `new Set<K>()`, `new Set<K>(xs)` with a `K[]`
   source (first-occurrence order), `new Set<K>(other)` with a
   `Set<K>` source, `size`, `add(k)` (returns the receiver), `has`,
   `delete`, `clear`, `forEach(f)` with `f` of `(k)`, and `for (const
   k of s)`.
6. **Mutation during traversal.** A `forEach` callback or a `for…of`
   body that adds, deletes, or clears an entry of the traversed
   container traps `map-modified` after that change. Divergence:
   `node` continues; it visits an entry added during the traversal and
   skips an entry deleted before its turn (measured: `forEach` over
   `{x}` that sets `y` visits `x,y`; `for…of` over `{1,2,3}` that
   deletes 2 at 1 visits `1,3`). The prior contract visits only the
   entries present at the start, without a trap; Ulua follows its own
   array rule (C6.9 `array-modified`). A `set` on a present key and
   an `add` of a present member change no entry and do not trap (as
   `node`).
   An `await` in the body of a `for…of` over a `Map` or `Set` is
   rejected (C6): a suspended traversal outlives its run, and a failed
   run of another export would reset its count.
7. **Rejected** (C6): `entries()`, `for…of` over a `Map` (each needs a
   tuple type), `new Map([[k, v]])`, a `forEach` callback with the
   collection parameter, `keys()` or `values()` outside the subject of
   `for…of`, `WeakMap`, `WeakSet`, a member read as a value, `Map` and
   `Set` as values, and the set algebra and `Map.groupBy` (a later
   revision).
8. **Limits.** Each prelude loop (compaction, copy, traversal) has the
   interrupt points of §5.19 and its allocations are charged.
9. **Corpus.** `a77` (map battery with number and string keys, NaN and
   `-0`, `getOr`, `size`, `clear`), `a88` (number keys outside the
   `int` range, rule 1a), `a78` (insertion order with
   overwrite, delete, re-insert; `keys()`, `values()`, `forEach`), `a79`
   (set battery, array source, identity keys of a class), `a82` (a
   `set` on a present key during a traversal), all `js-comparable:
   yes`; `t13` (`forEach` that adds an entry traps
   `map-modified`; `js-comparable: no C6`); `r159` (a function key), `r160`
   (`entries()`), `r161` (`for…of` over a `Map`), `r162` (`get` on a
   `V | null` map).
10. **Exit.** The corpus equals its goldens; `node` agrees on `a77` to
    `a79`; a randomized differential test of 10^4 operation sequences
    against `node` agrees on every observable result; the §5.19,
    analyzer, and P6.3 tests pass; the cost of each operation is
    measured and recorded.

## 9. P7.5 String case and code points

1. **Case.** `s.toUpperCase()` and `s.toLowerCase()` apply Unicode
   default case conversion with the special casing of Unicode 17.0
   (one-to-many results, final sigma), as the prior contract does
   (M5). The implementation is a native helper (form 4) over the Rust
   standard library `str::to_uppercase` and `str::to_lowercase` of the
   pinned toolchain (`rust-toolchain.toml`; `char::UNICODE_VERSION` is
   17.0). Measured (P7.5 contract): over every code point, the 3,037
   code points that either mapping changes give the same results as
   `node` v24.18.0 (Unicode 17.0), and 9 context rows (final sigma,
   `İ`, `ﬀ`, `ǅ`) are equal. A toolchain or `node` upgrade that
   changes the Unicode version changes the results; the tracking note
   records the version pair.
2. **Code points.** `String.fromCodePoint(cp)` with one `number`
   argument gives the UTF-8 string of `cp`. A `cp` that is not an
   integer in 0 to 0x10FFFF, or that is a surrogate (0xD800 to 0xDFFF),
   traps `invalid-argument`. Divergence: `node` throws `RangeError`
   outside the range and gives a lone surrogate for a surrogate.
   `String.fromCharCode(c)` with one `number` argument converts `c` as
   ToUint16; a result in the surrogate range traps `invalid-argument`
   (C5.1: UTF-8 has no lone surrogate). Divergence: `node` gives a lone
   surrogate, and two such calls concatenated form a pair there.
3. **Arity.** Each member takes exactly one argument (§0.7); the
   variadic forms are rejected.
4. **Charge.** A case call charges `NATIVE_CALL_STEPS` plus one step
   per `NATIVE_BYTES_PER_STEP` bytes of its argument; the result can be
   longer than the argument (`ß` gives `SS`) and is charged to
   `max_memory_bytes`.
5. **Rejected** (C5): `toLocaleUpperCase`, `toLocaleLowerCase`,
   `localeCompare`, `normalize`, `String.raw`, a member read as a
   value.
6. **Corpus.** `a80` (case over ASCII, Latin, Greek final sigma,
   German sharp s, Turkish dotted I, ligatures), `a81` (code points
   from ASCII to the astral planes), both `js-comparable: yes`; `t14`
   (`fromCodePoint(0x110000)`), `t15` (`fromCharCode(0xD800)`), both
   `js-comparable: no C5`; `r163` (`fromCharCode` with two arguments),
   `r164` (`toLocaleUpperCase`). `r18` retires: `toUpperCase` is
   accepted.
7. **Exit.** The corpus equals its goldens; `node` agrees on `a80` and
   `a81`; a unit test compares both mappings with a `node` table over
   every code point; the §5.19 and P6.3 tests pass.

## 10. P7.6 `JSON`

The contract of the owner's prior project (its §13), with the changes
that an Ulua rule forces. Measured with `node` v24.18.0 (P7.6
contract).

1. **`JSON.stringify(value)`** with one argument. The checker requires
   a serializable type and the emitter writes a serializer for it (no
   run-time type data: the static type is the value's type, as in the
   prior contract §13.1). Serializable: `number`, `string`, `boolean`,
   `T | null` (`null` gives `null`), `T[]`, a record, and a class (its
   fields; no method). Rejected (C20): a function type, `Map`, `Set`,
   a promise, an Error-family value, the absent type `T?`, and a type
   that contains one of them. `node` gives `{}` for a `Map` and a
   `Set` and omits a function field: a silent loss, as the prior
   contract states. The `replacer` and `space` arguments are rejected
   (a later revision).
2. **Field order (D34).** A record or class writes its fields in the
   declaration order of its type. An object literal whose type is, or
   relates to (D19), a record type that a `JSON.stringify` call can
   reach (through fields and elements) lists its fields in that
   declaration order; another order is rejected (C20). Reason: `node`
   writes the order of the literal (M4), and a Luau table keeps no
   order. Divergence: a value from `JSON.parse` writes in declaration
   order; `node` writes the key order of the parsed text.
3. **Values.** A number follows C4.5 (`-0` gives `0`). NaN and the
   infinities trap `json-number`, as the prior contract decides (§13.3:
   `node` writes `null`, a silent loss; the value that comes back is
   not the value that went in). A string escapes `"` and `\`, the five
   short escapes `\b \t \n \f \r`, and every other code point below
   U+0020 as a lowercase `\u00xx`; `/`, U+007F, U+2028, and U+2029 stay
   unescaped (measured).
4. **Cycles.** If the field graph of the type can reach the type again,
   the serializer keeps a visited set and throws `TypeError` on a
   revisit, as `node` does (measured). The prior contract traps; Ulua
   has catchable exceptions (C17), so it follows `node`. A type whose
   graph cannot cycle carries no visited set.
5. **Depth.** A serializer or a parse deeper than `MAX_JSON_DEPTH`
   (128, the prior contract's value) stops: `stringify` traps
   `limit:json-depth` (a resource limit, outside the comparison,
   invariant 5); `parse` throws `SyntaxError`, as the prior contract
   treats deep input as malformed. The script can catch it, so it is a
   named divergence (C20), not a limit: `node` parses 200 levels and
   gives the value.
6. **`JSON.parse(text)`** needs a target type from its context (an
   annotation, a parameter, a result, a field); a call with no
   contextual type is rejected (C20). The target follows rule 1, except
   that a class is not a target (its constructor would not run). A
   parse is a native helper (form 4) that reads the text with the
   ECMAScript JSON grammar, then a check that the emitter writes per
   target type converts the value.
   - Malformed text throws `SyntaxError` (as `node`).
   - Well-formed text that does not match the target throws
     `TypeError`: a missing field, a value of another type, `null` for
     a type without `null`. Divergence: `node` returns the value.
   - A field that the target does not declare is ignored. Divergence:
     a later `stringify` of the value omits it; `node` keeps it
     (measured: `{"x":1,"y":2}` parsed as `{ x: number }` gives
     `{"x":1}` here and `{"x":1,"y":2}` in `node`).
   - A duplicate key takes the last occurrence (as `node`).
   - A number outside the finite range (`1e400`) throws `TypeError`, by
     rule 3 (the prior contract §13.4). Divergence: `node` gives
     `Infinity`.
   - A `\u` escape of a lone surrogate throws `SyntaxError`: UTF-8 has
     no form for it (C5.1); a surrogate pair decodes to its code point.
     Divergence: `node` gives a one-unit string.
7. **Charges.** `stringify` is emitted Luau with the interrupt points
   of §5.19. `parse` charges `NATIVE_CALL_STEPS` plus one step per
   `NATIVE_BYTES_PER_STEP` bytes of the text; its values are charged to
   `max_memory_bytes`.
8. **Corpus.** `a83` (a `stringify` battery per serializable family
   with the escape set), `a84` (a `parse` battery: records, arrays,
   escapes, a surrogate pair, an extra field, a duplicate key, `-0`, a
   round trip), `a85` (malformed text throws `SyntaxError`; a cycle
   throws `TypeError`), all `js-comparable: yes`; `a86` (a mismatch
   throws `TypeError`, `1e400` included; `js-comparable: no C20`);
   `t16` (NaN traps `json-number`; `js-comparable: no C20`); `r165` (a
   `Map` value), `r166` (a function field), `r167` (a parse without a
   target), `r168` (a literal in another field order), `r169` (a
   `space` argument). A runtime test covers the depth bound at 128 and
   at 129 for both members.
9. **Exit.** The corpus equals its goldens; `node` agrees on `a83` to
   `a85`; a differential test of random values of 10 record and array
   types against `node` (`stringify`, and `parse` of the `node` text)
   agrees; a hostile-input test (deep nesting, long strings, many
   keys, every malformed prefix of a valid text) stays inside the
   limits; the §5.19 and P6.3 tests pass; the throughput of each member
   is measured and recorded.
