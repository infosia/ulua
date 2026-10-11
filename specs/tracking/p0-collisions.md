# P0 collision measurements

Status: measured 2026-10-11.

## Setup

- `node` v24.18.0.
- Luau CLI (`luau`) built in Release from the revision in
  `luau-revision.txt` (0.738).
- Command: `LUAU=<luau CLI> sh tools/probes.sh`. Each probe in
  `tools/probes/` has a `.mjs` file and a `.luau` file that compute the
  same values.

## Results

### `%` (probe `remainder`)

| Operands | node `%` | Luau `%` | Luau `math.fmod` |
|---|---|---|---|
| -7, 3 | -1 | 2 | -1 |
| 7, -3 | 1 | -2 | 1 |
| -4, 2 | -0 | 0 | -0 |
| 5.5, 2 | 1.5 | 1.5 | 1.5 |
| 5, 0 | NaN | NaN | NaN |
| Infinity, 2 | NaN | NaN | NaN |
| 2, Infinity | 2 | NaN | 2 |
| -2, Infinity | -2 | NaN | -2 |

`math.fmod` agrees on every row. Luau `%` disagrees on five rows.

### `**` (probe `pow`)

| Operands | node `**` | Luau `^` |
|---|---|---|
| 1, NaN | NaN | 1 |
| -1, Infinity | NaN | 1 |
| 1, Infinity | NaN | 1 |
| NaN, 0 | 1 | 1 |
| -8, 1/3 | NaN | NaN |
| 0, -1 | Infinity | Infinity |
| -0, -1 | -Infinity | -Infinity |

Luau `^` follows C `pow`. It disagrees when the base is ±1 and the
exponent is NaN or infinite.

### Number to string (probe `tostring`)

`String(n)` in node and `tostring(n)` in Luau agree on 19 of 25 values,
including `0.30000000000000004`, `1e+21`, `5e-324`, and
`1.7976931348623157e+308`. They disagree on these values:

| Value | node | Luau |
|---|---|---|
| -0 | `0` | `-0` |
| 1e-7 | `1e-7` | `1e-07` |
| 1.5e-7 | `1.5e-7` | `1.5e-07` |
| NaN | `NaN` | `nan` |
| Infinity | `Infinity` | `inf` |
| -Infinity | `-Infinity` | `-inf` |

M4 (`plan.md` §7) extends this to a random sample.

### Function identity (probe `closure-identity`)

| Form | node | Luau |
|---|---|---|
| `make() === make()`, closure with no upvalue | false | true |
| two closures from two loop iterations | false | true |

The Luau compiler reuses a closure that captures no upvalue.

### Absent table elements (probe `nil-in-table`)

| Form | node `length` | Luau `#` |
|---|---|---|
| `[1, null, 3]` | 3 | 3 |
| `[1, 2, null]` | 3 | 2 |
| `[1, 2, 3]`, then element 1 set to null | 3 | 3 |
| eight elements, then element 3 set to null | 8 | 8 |

Luau `#` returns a border. With a nil element the result depends on the
table layout. It disagrees on the second row.

### Out-of-range index (probe `index-read`)

| Read on `[10, 20]` | node | Luau, lowered to `xs[i + 1]` |
|---|---|---|
| `xs[5]`, `xs[-1]`, `xs[0.5]` | `undefined` | `nil` |
| `xs[0.9999999999999999]` | `undefined` | `20` |
| `xs[-1e-17]` | `undefined` | `10` |

For the last two rows `i + 1` rounds to an integer. A check of the Luau
result for `nil` does not detect them. The range check must test `i`
before the addition.

### Strings (probe `strings`)

| Form | node | Luau |
|---|---|---|
| length of `"héllo"`, `"😀"`, `"日本"` | 5, 2, 2 | 6, 4, 6 (`#`) |
| `"\uFFFF" < "😀"` | false | true |
| `"z" < "é"` | true | true |
| upper case of `"é"`, `"straße"` | `É`, `STRASSE` | `é`, `STRAßE` (`string.upper`) |
| lower case of `"ÀB"` | `àb` | `Àb` (`string.lower`) |
| trim of `" \u00a0x\u3000"` | `x` | ` x　` (`gsub` with `^%s+`) |
| `"a,b,,c".split(",")` | `a`, `b`, ``, `c` | `a`, `b`, ``, `c` (`string.split`) |
| `"日本語".split("本")` | `日`, `語` | `日`, `語` |
| `"hé".split("")` | 2 pieces: `h`, `é` | 3 pieces: `h` and two single bytes |
| `"héllo".includes("ll")`, `includes("")` | true, true | true, true (`string.find`, plain) |
| `startsWith("hé")`, `endsWith("llo")` | true, true | true, true (`string.sub` compare) |
| `"héllo".startsWith("l", 2)` | true | false (byte position 3 is a byte of `é`) |
| `("\uD83D" + "\uDE00") === "😀"` | true | no UTF-8 form for a lone surrogate |
| `"a" + 1e-7`, `"a" + -0` | `a1e-7`, `a0` | `a1e-07`, `a-0` (`..`) |

### Evaluation order (probe `evaluation-order`)

`n + inc()`, where `inc` increments `n` from 0 and returns it: node 1,
Luau `n + inc()` 2. Luau reads the local `n` after the call.

### Literals (probe `literals`)

| Form | node | Luau |
|---|---|---|
| `0xffffffffffffffffff === 4722366482869645213696` | true | false (saturates) |
| `0o17` | 15 | compile error (`loadstring` gives nil) |
| raw CR, raw NUL inside a quoted string | — | compile error for each |
| byte length of `"a\r\nb"` | 4 | 4 quoted; 3 as a long bracket with raw CR LF |

### Compiler limits (probe `compile-limit`)

| Module form | Luau compiles |
|---|---|
| 201 `local` bindings with non-constant initializers (`id(i)`) | no (`Out of local registers`) |
| 201 `local` bindings with constant initializers | yes (constant folding) |

node runs both forms.

### Further Luau limits (probe `luau-limits`)

| Chunk | Luau compiles |
|---|---|
| a closure that captures 200 bindings | yes |
| a closure that captures 201 bindings | no (`Out of upvalue registers`) |
| a flat `x + x + ...` chain of 200 terms | yes |
| a flat chain of 3000 terms | no |
| 32,767 closures in one function | yes |
| 32,769 closures in one function | no |

`0xffffffffffffffffff == 2^64` is `true` in Luau: the literal saturates
to 2^64.

## Consequences

The rules in `specs/blocks/collisions.md` cite these rows.
