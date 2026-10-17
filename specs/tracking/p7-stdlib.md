# P7 standard library

Status: P7.0 measured 2026-10-17; two owner decisions open (below). Contract: `blocks/stdlib.md` Rev 0.

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
