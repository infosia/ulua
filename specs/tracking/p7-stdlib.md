# P7 standard library

Status: P7 COMPLETE 2026-10-17 (P7.0 to P7.6; Phase Review round 1). Contract: `blocks/stdlib.md` Rev 11.

## Slices

`blocks/stdlib.md` §3. P7.0 (measurement round) opens first.

## Measurements (planning probe)

Pinned VM (`luau` CLI at `luau-revision.txt`) and `node` v24.18.0.
A probe, not the M1 measurement: one input per row.

| Form | Luau | `node` |
|---|---|---|
| `round(2.5)`, `round(-2.5)` | 3, -3 | 3, -2 |
| `round(0.49999999999999994)`, `round(-0.4)` | 0, -0 | 0, -0 |
| `max(NaN, 1)`, `max(1, NaN)` | NaN, 1 | NaN, NaN |
| `max(0, -0)`, `max(-0, 0)` | 0, -0 | 0, 0 |
| `min(0, -0)`, `min(-0, 0)` | 0, -0 | -0, -0 |
| `sign(-0)`, `sign(NaN)` | 0, 0 | -0, NaN |
| `abs(-0)`, `floor(-0.5)`, `ceil(-0.5)` | 0, -1, -0 | 0, -1, -0 |
| `1 ** NaN`, `(-1) ** Infinity`, `NaN ** 0` (Luau `^`) | 1, 1, 1 | NaN, NaN, 1 |
| `log2(8)` (`math.log(8, 2)`), `log10(1000)`, `log(-0)` | 3, 3, -inf | 3, 3, -Infinity |
| `atan2(-0, -1)`, `sqrt(-0)`, `sqrt(-1)` | -π, -0, NaN | -π, -0, NaN |
| `toFixed`: 2.5 at 0, 0.5 at 0, 1.005 at 2, 0.25 at 1 (`string.format("%.nf")`) | 2, 0, 1.00, 0.2 | 3, 1, 1.00, 0.3 |

Absent from the Luau `math` library: `trunc`, `cbrt`, `hypot`, `log2`,
`expm1`, `log1p`. Present: `sinh` (and the other members of the probe).

Consequence: `round`, `max`, `min`, `sign`, and `pow` need form 2 or 3
of `stdlib.md` §0.2; `toFixed` needs a helper (`%.nf` rounds the
binary value half to even, `toFixed` picks the larger `n` on a tie).

## P7.0 measurements

Pinned VM (`luau` CLI, `luau-revision.txt`), `node` v24.18.0 (V8
13.6.233.17, Unicode 17.0, ICU 78.3), macOS arm64. Scratch harnesses;
no production change.

### M1 `Math`

100,050 single inputs (50 edges, 40,000 uniform in [-10, 10], 40,000
log-uniform magnitudes 1e-20 to 1e20, 20,000 random finite bit
patterns); 72,500 pairs (2,500 edge pairs). Bitwise comparison; any
NaN equals any NaN.

| Group | Members | Result |
|---|---|---|
| Builtin equal | `abs`, `ceil`, `floor`, `sqrt` | 0 differences |
| Helper equal | `round`, `sign`, `trunc`, `max`, `min`, `clz32`, `imul`, `fround` (Luau, `string.pack("<f")` for `fround`) | 0 differences |
| Builtin, last bits | `acos` 5,450, `asin` 439, `atan` 2,416, `cos` 3,276, `sin` 3,061, `tan` 31,410, `cosh` 10,366, `sinh` 13,014, `tanh` 5,038, `exp` 4,350, `log` 1,472, `log10` 3,060, `atan2` 9,544 of the inputs | 1 or 2 ulp |
| Builtin with edge rewrites | `pow` (`a ^ b` with the NaN rule for `b` NaN and for `\|a\| = 1`, `b` infinite) | 2 of 72,500 at 1 ulp |
| Formula, wrong | `cbrt`, `expm1`, `log1p`, `asinh`, `acosh`, `atanh` (algebraic formulas over builtins) | up to 4.5e15 ulp |
| Rust `libm` 0.2.16 | the 22 members above | `log2` 0; others 11 to 14,824 of the inputs at 1 or 2 ulp |

The Luau builtins call the libm of the host OS. Consequence: a result
of the "last bits" group depends on the OS, so a golden would differ
between the macOS, Linux, and Windows gates (D28). `node` uses the
fdlibm-derived code of V8 *(docs)*, which neither the host libm nor
Rust `libm` reproduces bit for bit on these inputs.

### M2 Number formatting and parsing

`toFixed` over 6,020 values (ties `k / 2^m`, magnitudes to 1e12, the
edges) times 7 digit counts, 42,140 rows. `string.format("%.nf")` on
the pinned VM and Rust `format!("{:.n}")` give the same 42,140
strings. Both differ from `node` in three classes only: 743 exact
ties (`(2.5).toFixed(0)` is `2` against `3`; ECMAScript picks the
larger `n`, which rounds half away from zero), 7 `-0` rows (`-0`
against `0`), 7 rows at or above 1e21 (ECMAScript gives `String(x)`).
`tonumber` against `Number` over 40,007 decimal strings (shortest and
`toPrecision` forms, the subnormal and overflow edges): 0 differences.
`toPrecision`, `toExponential`, `toString(radix)`, and `parseInt` are
measured in P7.3.

### M3 `Map` and `Set`

| Operation | plain table (int / string keys) | ordered prelude table | ratio |
|---|---|---|---|
| `set` | 83.7 / 59.7 ns | 111.5 / 88.9 ns | 1.3 to 1.5 |
| `get` | 28.8 / 20.1 ns | 33.8 / 26.6 ns | 1.2 to 1.3 |
| `delete` (with compaction at half live) | 30.9 / 23.1 ns | 74.9 / 63.9 ns | 2.4 to 2.8 |
| iteration per live entry | 12.7 / 12.7 ns | 18.9 / 18.8 ns | 1.5 |

200,000 keys, best of 5, `-O2`. A plain table iterates in hash order
(`5,3,100,1,2` gives `1,2,3,5,100`); `node` iterates in insertion
order. A NaN key fails in Luau (`table index is NaN`); `node` accepts
it. `-0` and `0` are one key in both.

### M4 `JSON`

`node`: `JSON.stringify({y:1,x:2})` is `{"y":1,"x":2}` and
`JSON.stringify({x:2,y:1})` is `{"x":2,"y":1}`: the order is the
order of the literal. A class instance gives the field declaration
order. NaN and the infinities give `null`, `-0` gives `0`, `1e21`
gives `1e+21`, U+0001 gives `\u0001`. `JSON.parse("[1e400, -0]")`
gives `[Infinity, -0]`; a malformed text throws `SyntaxError`. A
record is a Luau table with no field order, so the emitter cannot
recover the order of the literal at run time.

### M5 Case mapping

`node`: `toUpperCase` changes 1,580 code points (102 to more than one
code point, `ß` to `SS`); `toLowerCase` changes 1,488 (1 to two code
points, `İ`); final sigma depends on context (`ΑΣ` to `ας`). Luau
`string.upper("abcé")` is `ABCé` (ASCII only). The mapping follows the
Unicode version of `node` (17.0).

### M6 Absence

`tsc` with the project options: `const r = xs.find(f)` has type
`number | undefined`; `r == null`, `r ?? 0`, `r != null` narrowing,
`q?.x ?? -1` are clean; `r === null` is clean (no TS2367); `const
t: number | null = xs.find(f)` gives TS2322. `node`: `r == null` is
`true`, `r === null` is `false`, `r ?? 0` is `0`.

## Owner decisions open

1. M1: the implementation of the "last bits" group and of `pow`,
   `cbrt`, `expm1`, `log1p`, `asinh`, `acosh`, `atanh`, `hypot`,
   `log2`. The prior contract calls the Rust `f64` methods, which call
   the libm of the host OS (read in its runtime `math.rs`).
2. M4: the field order of a record in `JSON.stringify`. The prior
   contract writes the declaration order; its records do not relate
   structurally, Ulua records do (D19).

M5 follows the prior contract: Unicode default case conversion with
the special-casing table, as a native helper (form 4 of `stdlib.md`
§0.2), at a pinned Unicode version.

## P7.1 `Math`

Contract: `stdlib.md` §5 (Rev 2), `abi.md` Rev 10, collisions.md C4.4,
C4.9. Corpus: `a69` (equal to `node`), `a70` (golden from Rust `libm`
outside the product), `a71` (golden from a reference xoshiro256++),
`r149` to `r151`; `r46` retired. Red before: all six gave C11.

Failures before the fix:

- `a70` gave `+pi` for row 27: the source generator wrote `-0` as `0`
  (`String(-0)`), while the golden used `atan2(-0, -1)`. The source now
  reads `-0`.
- `math-native-loop` gave the §5.19 ratio 6.152 against 6 with no extra
  step charge.

Measurements (Release, macOS arm64): a builtin `floor` call 11.1 ns, a
native `sin` call 30.3 ns, `random` 13.6 ns. `NATIVE_CALL_STEPS`
selection (§5 rule 5), largest Math-shape ratio: 1 gives 2.747, 2
gives 1.748, 4 gives 1.664, 8 gives 1.719; the value is 1. A boundary
test: a budget of 5 steps stops with `limit:steps` at the third native
charge. `luau-analyze` on `a69` to `a71`: 0 errors.

Gates on the final code: `cargo test -p ulua_compiler` 228 passed;
`math_tests` 42 passed; `gate.sh full` PASS (226.8 s); `gate.sh
timing` PASS (421.7 s): §5.19 largest 5.894 (`async-reject-resume`,
not a Math shape; Math shapes 3.094, 1.880, 1.009), analyzer largest
11.608, P6.3 within every bound (primes 1.186, sort 3.520).

## P7.2 `find`, `findLast`, the absent type

Contract: `stdlib.md` §6 (Rev 3), collisions.md C2, C6.9. Corpus:
`a72` (equal to `node`), `a73` (`js-comparable: no C2`), `r152` to
`r155`; `r24` retired. Red before: all seven gave C6. The checker
holds `T?` as its own type (`Absent`), apart from `T | null`; the
helper returns the element or `nil`.

Gates on the final code: `cargo test -p ulua_compiler` 231 passed;
`gate.sh full` PASS (152.4 s); `gate.sh timing` PASS (399.0 s): §5.19
largest 5.814 (`boolean-template`; `find` 1.914, `findLast` 1.921),
analyzer largest 11.650, P6.3 within every bound.

A process failure during the slice: Red P7.3 entries entered the
working tree while the P7.2 gates ran, and `gate.sh full` failed on
them. They moved to the scratchpad until the P7.2 commit.

## P7.3 `Number`

Contract: `stdlib.md` §7 (Rev 6), collisions.md C4.10. Corpus: `a74`
to `a76` (equal to `node`), `t12`, `r156` to `r158`. Red before: all
seven (C11, C1).

Failures before the fix (each changed the contract):

- With `NATIVE_CALL_STEPS` only, `toFixed(100)` in a loop gave the
  §5.19 ratio 30.6 (Rev 5).
- A fixed format charge needed 512 steps per call and still gave 7.162
  in the timing gate for the longest binary fraction of
  `toString(radix)`, which writes more than 1,000 digits (Rev 6).

The charge: a parse call `NATIVE_CALL_STEPS` + one step per byte
(`NATIVE_BYTES_PER_STEP` 1; 12 long-string shapes, largest 3.964); a
format call `NATIVE_CALL_STEPS + 32 + 3 × output bytes + ceil(|e| / 2)`
with `e = floor(log2(|x|))` (0 for zero and non-finite values);
`(1).toFixed(0)` costs 36 steps. The sweep (225 shapes: 4 value
classes × digits 0, 20, 100 and omitted × radixes 2, 3, 10, 36 × 4
members) stays at most 4.947 (`(1).toString(10)`); it is in
`crates/ulua/test-data/number-format-sweep.tsv`.

`node` tables (`crates/ulua/test-data/number-oracle.tsv`, 131,301 rows,
13.8 MB, regenerated by `number-oracle.mjs`): `parseFloat` 10,100,
`toFixed` 20,200, `toExponential` 20,200, `toPrecision` 10,100,
`toString` 20,200, each predicate 10,100: 0 differences. `parseInt`
10,100: 3,265 differences (C4.10).

Gates on the final code: `cargo test -p ulua_compiler` 233 passed;
`gate.sh full` PASS (238.5 s); `gate.sh timing` PASS (433.6 s): §5.19
largest 5.824 (`boolean-template`), analyzer largest 11.912
(`P22-array-wide-function-product`), P6.3 within every bound.

## P7.4 `Map` and `Set`

Contract: `stdlib.md` §8 (Rev 8), collisions.md C6.10 (Rev 50). Corpus:
`a77` to `a79`, `a82` (equal to `node`), `t13`, `r159` to `r162`. Red
before: all eight (C6, C11).

The representation: an opaque prelude table with an index table, key
and value arrays in insertion order, a NaN slot, a live count, and
compaction at half of the slots. The shim keeps a registry of active
traversals and clears it at a failed run boundary; `map-modified` is
shim status 19.

Failures before the fix (review round 1):

- An overwrite of a present key during a traversal trapped
  `map-modified`; `node` continues. The contract (Rev 8, C6.10 Rev 50)
  now states that an overwrite changes no entry; `a82` pins it and was
  Red with `trap:map-modified`.
- The checker used `unreachable!()` and guarded `.unwrap()` calls
  (core principle 5). They are total matches now.

`node` differential (`crates/ulua/test-data/map-oracle.tsv`, from
`map-oracle.mjs`): 10,000 sequences, 320,000 operations, 0 differences.

Cost (Release product VM, ns per operation, median of 5 × 100,000,
loop included), number keys / string keys: `set` 139.3 / 210.0, `get`
59.3 / 104.8, `has` 37.3 / 91.2, `delete` 256.5 / 286.0, iteration per
entry 34.5 / 34.6.

Gates: before round 1, `gate.sh full` PASS (163.9 s), `gate.sh timing`
PASS (409.6 s): §5.19 largest 5.807 (`boolean-template`), analyzer
largest 11.440, P6.3 within every bound (primes 1.178, sort 3.512).
After round 1 (the helper loops did not change): `cargo test -p
ulua_compiler` 237 passed, `p74_tables` 8 passed, `luau.mjs` 229
entries pass, `gate.sh quick` PASS (156.2 s).

## P7.5 String case and code points

Contract: `stdlib.md` §9 (Rev 9), collisions.md C5.4, C5.6 (Rev 51),
surface.md Rev 30. Corpus: `a80`, `a81` (equal to `node`), `t14`,
`t15`, `r163`, `r164`; `r18` retired. Red before: `a80`, `a81`, `t14`,
`t15` gave C5.

The four members are native calls of the `Math` table path. A case
call charges `NATIVE_CALL_STEPS` plus one step per byte, and allocates
a VM buffer of `3 × len + 4` bytes before the conversion; the Rust
result is bounded by that charged buffer. The per-scalar expansion
bound of 3 is checked over every scalar.

Version pair: Rust `char::UNICODE_VERSION` 17.0.0, `node` v24.18.0
(Unicode 17.0). `node` table (`crates/ulua/test-data/string-case-oracle.tsv`,
from `string-case-oracle.mjs`): 3,037 changed scalars and 9 context
rows, 0 differences.

Cost (Release, ns per call, median of 5 × 200,000, loop included),
upper / lower: `"Hello World"` 74.2 / 68.2, `"Straße ΟΣ İ ﬀ"` 137.5 /
199.7.

Gates on the final code: `cargo test -p ulua_compiler` 239 passed;
`gate.sh full` PASS (207.4 s); `gate.sh timing` PASS (434.0 s): §5.19
largest 5.929 (`boolean-template`; case shapes 2.155, 1.202), analyzer
largest 11.620, P6.3 within every bound (primes 1.169, sort 3.394).

## P7.6 `JSON`

Contract: `stdlib.md` §10 (Rev 10), collisions.md C20 (Rev 52),
corpus.md §6 `json-number`, surface.md Rev 31. Corpus: `a83` to `a85`
(equal to `node`), `a86` (`js-comparable: no C20`), `t16`, `r165` to
`r169`. Red before: all ten gave C11.

The form: the emitter writes a serializer and a target check per
static type; a Rust parser (`crates/ulua/src/json.rs`) gives grammar
events, and the shim (`native/shim/json.h`) builds the values through
the VM allocator.

Failures before the fix: the first `gate.sh timing` run gave the
compile-memory base 85,520 bytes against the bound 85,347. Removal of
redundant declaration metadata and empty type-cache entries gave
85,180.

`node` differential (`crates/ulua/test-data/json-oracle.tsv`, from
`json-oracle.mjs`): 10 record and array types, 320 `stringify` values
and 320 `parse` round trips, 0 differences. Runtime tests cover every
malformed prefix, long strings, many keys, memory limits, and depth 128
and 129 for both members.

Throughput (Release; a record with a 10,000-element number array, 100
calls, 4,889,800 bytes): `stringify` 18.7 MB/s, `parse` 34.5 MB/s.

Gates on the final code: `cargo test -p ulua_compiler` 244 passed;
`gate.sh full` PASS (190.5 s); `gate.sh timing` PASS (412.7 s): §5.19
largest 5.749 (`boolean-template`; JSON shapes 3.673, 4.466, 4.433,
1.217), analyzer largest 11.638, P6.3 within every bound (primes
1.188, sort 3.406).

## P7 Phase Review round 1

A fresh reviewer read `git diff 0c3a4d7^..HEAD` with probes under the
product binary and a C host. Result: 0 CRITICAL, 3 MAJOR, 3 MINOR.

- M1 (MAJOR): a failed run reset the traversal count of a `for…of`
  over a `Map` that a suspended task held; the count went to -1 and
  `map-modified` stopped. Measured: `each 1`, `each 4`, status 0;
  `node` gives `each 1`, `each 3`, `each 4`. Fix: `await` in such a
  body and a promise-result `forEach` callback are rejected (C6.10 Rev
  53, `r170`). With no suspended traversal, the reset at a failed run
  is exact.
- M2 (MAJOR): a field outside the parse target is dropped on a later
  `stringify`; no rule named it. Fix: C20.6 names it with both
  outputs; `a86` pins it.
- M3 (MAJOR): a parse deeper than 128 throws a catchable
  `SyntaxError`; `node` gives the value at 200 levels. Fix: C20.5
  names it; `a86` pins it.
- m1 (MINOR, open): a case call builds its result in a Rust `String`
  outside the VM allocator. The size is at most the charged VM buffer
  of `3 × len + 4` bytes, so peak host memory is at most twice the
  charge for the duration of the call.
- m2 (MINOR): `assert!` and `unwrap()` on the `number.rs` dispatch
  path. Fix: total forms that return -1.
- m3 (MINOR): `a84` did not stringify an extra field. Fix: `a86`.

Gates on the final code: `cargo test -p ulua_compiler` 247 passed;
`p74_tables` 9 passed; `luau.mjs` 245 entries pass; `gate.sh full`
PASS (primary run); `hygiene.sh` PASS. Timing: the emitted helper
loops did not change after the P7.6 timing run (§5.19 largest 5.749,
analyzer largest 11.638).

Open: m1; the analyzer ratio of `P22-array-wide-function-product`
stays from 11.44 to 11.91 against the bound 12 (one run gave 12.023 in
P6).

## P7.4 defect: number keys outside the `int` range

Status: OPEN 2026-10-18. Contract: `blocks/stdlib.md` Rev 12 §8 rule 1a.

Failure: `gate.sh sanitize` (P6.S7) stops in
`p74_tables::node_differential_ten_thousand_sequences` with UBSan
"is outside the range of representable values of type 'int'" at
`lvmexecute.cpp:779`. The key is `-inf`. It reproduces at 70479aa (before
P6.S7). Cause: `__ulua_table_slot` and `__ulua_table_set_0` index
`m.index` with the script key; the pinned VM converts a number key with
`int(d)` in `GETTABLE`, `SETTABLE`, and `luaH_get`. Severity: CRITICAL
(a script reaches undefined behaviour in the VM, invariant 2). The
P7 Phase Review did not run `gate.sh sanitize`.

Corpus: `a88` (Map and Set keys: both infinities, `±1e300`, `2^31`,
`2^31 - 1`, `-2^31`, `-2^31 - 1`, `±(2^31 + 0.5)`, `2^53`, `0.5`;
`js-comparable: yes`; `node` and the current build agree).
