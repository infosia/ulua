# Semantic collisions — decided rules

Status: Rev 39, 2026-10-12 (Rev 0 to Rev 11 the same day; P0 Phase
Review rounds 1 to 12 changed C1 to C6, C9, C10, C12, C14, C16 and added C11
to C16; Rev 13: C7 literal operands, P1 Phase Review round 3; Rev 14: C7
structural operand rule, round 4; Rev 15: C7 accepts `==` and `!=`;
Rev 16: the three resolutions and the divergence format, invariant 5;
Rev 17: C7 rule 4 as a flow fact, round 5; Rev 18: aliased conditions,
literal conditions, strict-mode early errors, round 6; Rev 19:
`ALIAS_TEST_BOUND`, round 7; Rev 20: nested bodies inherit the tested
set, P2.1; Rev 21: C1.4 function type equality, P2.1 review; Rev 22: C10.3
closures inside a declaration, C15 closures note, P2.1 review 2; Rev 23:
C6.3 divergence text, C7 array literal operands and `for…of` bindings,
P2.2 review; Rev 24: C6.9 array methods, P2.3; Rev 25: C7 type-based equality, D17; Rev 26: C4.6, C5 byte semantics, P2.4; Rev 27: C1.5 structural object
types (D19), C2 strict null tests, C7 object identity, P2.5; Rev 28: C1.6 `__proto__`, C16.4
function declarations, P2.5 implementation; Rev 29: C16.1, C16.6, C16.7,
P2.5 review; Rev 30: C16.7 join edges, P2.5 review 2; Rev 31: C16.8
unreachable points, P2.5 review 3; Rev 32: C16.9 edge ends, P2.5 review 4; Rev 33: C16.1 `?:` tests,
C16.9 logical tests, P2.5 review 5; Rev 34: C16.9 `??` tests and
contradicted references, P2.5 review 6; Rev 35: C12.4 store through an
element read, P2.5 review 7; Rev 36: C16.4 tests in nested bodies, P2.5 review 10; Rev 37: C13.3 to C13.5, C1.6 class members, P2.6; Rev 38: C2 optional calls in arrow bodies, P2.6 review; Rev 39: C17 exceptions, C16.9 `throw`, P2.7; Rev 40: C16.8 arrows at unreachable points, P2.7 review 3; Rev 41: C18 host functions, P3; Rev 42: C19 async functions and promises, P4.1; Rev 43: C19.10 to C19.15 then, catch, finally, Promise.all, map, P4.2; Rev 44: C19.16 async host functions, P4.3; Rev 45: C4.3 native remainder, P6.S6; Rev 46: C4.4 `Math.pow`, C4.9 `Math`, C11 `Math`, P7.1; Rev 47: C2 the absent type, C6.9 `find` and `findLast`, P7.2; Rev 48: C4.10 `Number`, P7.3; Rev 49: C6.10 `Map` and `Set`, P7.4; Rev 50: C6.10 an overwrite during a traversal, P7.4 review; Rev 51: C5.6 case and code points, P7.5; Rev 52: C20 JSON, P7.6; Rev 53: C6.10 `await` in a traversal, C20.5 and C20.6, P7 Phase Review).

This file resolves each collision between the TypeScript surface and
the native Luau representation (`CLAUDE.md` invariants 4 and 5). Stock
`tsc` accepts a superset. This compiler narrows it. A form that `tsc`
cannot police is policed here, with a diagnostic that names the rule.
Measurements are in `specs/tracking/p0-collisions.md`.

A rule resolves a collision in one of three ways: it rejects the form,
it lowers the form to a fixed Luau expression that agrees with `node`,
or it names a divergence. A divergence states the measured output of
the Ulua run and of `node` for each input class where they differ. A
trap where `node` continues with a value outside the static type is a
divergence; its trap entries compare the output before the trap.
Entries of any other divergence state `js-comparable: no <id>`.

Each rule ends with the corpus entries that it names. An entry names its
rules in its `rule:` header. `tools/witness/corpus.mjs` checks that the
two lists agree.

## C1. Value representation

| TypeScript type | Luau value |
|---|---|
| `number` | `number` (binary64) |
| `boolean` | `boolean` |
| `string` | `string`, valid UTF-8 |
| `T[]` | a table with the elements at keys 1 to n and no other keys |
| object type, class instance | a table |
| function type | `function` |
| `null` | `nil` |

No other value appears. A lowering does not add a field, a sentinel, a
wrapper, or a second encoding to a value.

The checker decides assignability itself. It does not take the `tsc`
relation. The relation applies at every value flow (surface.md §3:
arguments, initializers, returns, literal entries, assignments) and at
every depth: array elements, union members, function parameters and
results, and object fields.

1. A `number`, `boolean`, `string`, array, or function type is never
   assignable to an object type. `tsc` accepts `len("héllo")` for
   `len(v: { length: number })`, and `length` is not a field of a Luau
   string.
2. `T` is assignable to `T | null`.
3. Array element types must be identical. `tsc` accepts `string[]` as
   `{ length: number }[]` by covariance.
4. A function type is assignable only to an equal function type: the
   same parameter types, the same result type, and the same number of
   parameters with a default. The branches of `?:` have one type, so
   two function branches have equal types. `tsc` relates a function
   with fewer required parameters to a function type with more; a call
   through the second type can then omit an argument that the first
   needs (`b ? g : f` with `g(a: number = 7)` and `f(a: number)`).
5. Object types are structural (D19): two object types relate when
   they have the same field names and identical field types; an extra
   field, a missing field, or an optional field is rejected. A class is
   nominal. An object type and a class never relate, in either
   direction. A call of a field of
   function type reads the field and calls it. A method call is a
   direct call of the method's generated function with the receiver as
   its first argument (C14). A class instance seen through an object
   type, or an object literal seen through a class type, selects the
   wrong call form.
6. A field named `__proto__` is rejected, in an object type, in an
   object literal, and as a class field or method. `node` reads `{ __proto__: 1 }.__proto__` as the
   prototype object and prints `[object Object]`. The Luau table field
   holds `1`.

Accept: `a01`, `a05`, `a09`, `a15`, `a16`, `a49`. Reject: `r19`, `r43`, `r53`, `r84`,
`r54`, `r102`, `r113`.

## C2. Absence — `T | null` only

- `T | null` is the only union. Every other union is rejected.
- `null` lowers to `nil`.
- `undefined` is rejected everywhere: a type, an expression, an
  optional parameter `x?: T`, an optional property `x?: T`.
- A parameter with a default is accepted if its type excludes `null`.
  The default fills the value, and no `undefined` is observable. With
  a `T | null` type, an omitted argument and `null` are both `nil`, and
  JavaScript applies the default to the first only.
- `void` is a return type only. An expression of type `void` is
  accepted only as an expression statement.
- A null test is `x == null`, `x != null`, `x === null`, or
  `x !== null`, with `x` of type `T | null`. D16 keeps every
  `undefined` out of accepted programs and of the library, so `===` and
  `==` agree with `node` here. A null test on a type without `null` is
  rejected.
- `a ?? b` is accepted when `a` has type `T | null`. It lowers to an
  if-expression on `a ~= nil` with `a` evaluated once, never to
  `a or b`: `false ?? true` is `false` in JavaScript, and `false or
  true` is `true` in Luau.
- An optional chain has exactly one optional step and no step after it:
  `a?.f` or `a?.m(args)` (surface.md §3). It is accepted in two
  positions only: `a?.f` as the left operand of `??`, and `a?.m(args)`
  as an expression statement. An arrow expression body is not an
  expression statement: `(): void => r.d?.m()` gives TS2322 when `m`
  returns a value. In any other position its type includes
  `undefined`. A step after the optional step, as in `a?.f.g`, is
  rejected: JavaScript skips the whole chain when `a` is `null`.

- **The absent type (D32, stdlib.md §6).** `find` and `findLast` give
  `T?`: `nil` on a miss, where `node` gives `undefined`. `==`, `!=`,
  `??`, and `?.` agree with `node`. Divergence: on a miss `x === null`
  is `true` here and `false` under `node`, and `x !== null` is `false`
  here and `true` under `node`.

Accept: `a08`, `a14`, `a15`, `a40`, `a72`, `a73`. Reject: `r02`, `r03`, `r04`, `r12`, `r13`,
`r14`, `r48`, `r63`, `r118`, `r152`, `r153`, `r154`, `r155`, `r162`. Retired by P2.5: `retired:r05-strict-null-equality`.

## C3. Truth tests are `boolean`

`if`, `while`, `for`, `?:`, `!`, `&&`, and `||` require `boolean`
operands. A truth test on any other type is rejected. A comparison
states which value is false: `n !== 0`, `s !== ""`, `x != null`.

Reason: JavaScript treats `0`, `NaN`, `""`, and `null` as false. Luau
treats only `nil` and `false` as false.

`c ? a : b` lowers to the Luau if-expression `if c then a else b`, never
to `c and a or b`, which gives `b` when `a` is `false`. `&&` and `||`
on booleans lower to `and` and `or`.

Accept: `a06`, `a07`, `a10`. Reject: `r07`.

## C4. Numbers

1. `number` is the only numeric type. BigInt literals and the `bigint`
   type are rejected. `-`, `*`, `/`, unary `-`, and the comparisons take
   number operands only (surface.md §3) and lower to the Luau operators
   with the same name. `+` on two numbers lowers to Luau `+`; `+` on two
   strings lowers to Luau `..` (C5). Unary `-` on a string or a boolean is
   `NaN` or a number in JavaScript and an error in Luau.
2. A lowering preserves NaN behavior. The emitter never rewrites
   `!(a < b)` as `a >= b`.
3. `%` lowers to `math.fmod`. Luau `%` floors and disagrees with
   JavaScript on 5 of 8 measured rows; `math.fmod` agrees on all 8.
   Where integer facts prove a dividend of `+0` or an integer up to
   `2^53` and a finite integer divisor `>= 1`, `%` lowers to the
   native Luau `%`, which gives the same value (`pipeline.md` §5.25).
4. `**` is rejected. `Math.pow` is accepted (stdlib.md §5). Luau `^` follows C `pow` and gives
   `1` where JavaScript gives `NaN` for a base of ±1 with a NaN or
   infinite exponent. No lowering is decided.
5. A number in a template substitution or in `String(n)` follows the
   JavaScript format. Luau `tostring` and `..` disagree on `-0`, NaN,
   the infinities, and exponents below `1e-6`. The lowering is a fixed
   function: NaN gives `NaN`, the infinities give `Infinity` and
   `-Infinity`, zero gives `0`, and every other value gives `tostring`
   with the leading zeros of the exponent removed. It agrees with node
   on 800,019 of 800,019 sampled values (M4). The golden is the
   JavaScript format.
6. `+` and `+=` with one `string` operand and one `number` or `boolean`
   operand convert the other operand as a template substitution does
   (rule 5 for a number; `true` or `false`) and concatenate. The
   operands are evaluated in order (C12). An operand of any other type
   is rejected (rule 8). (P2.4; the owner's prior project accepts the
   same form.)
7. The bitwise operators are rejected. No lowering is decided.
8. A template substitution has type `number`, `string`, or `boolean`.
   `String(x)` takes a `number`. JavaScript formats `null` as `null`,
   an object as `[object Object]`, and an array by its elements. Luau
   `tostring` gives `nil` and a table address.

9. `Math` (stdlib.md §5). The form-4 members give the values of Rust
   `libm` 0.2.16; `node` gives the values of V8. Measured over 100,050
   inputs (72,500 pairs for `atan2`, `hypot`, `pow`), the number of
   1- or 2-ulp differences: `acos` 24, `acosh` 1,221, `asin` 71,
   `asinh` 529, `atan` 57, `atanh` 23, `atan2` 280, `cbrt` 8,399, `cos`
   961, `cosh` 229, `exp` 11, `expm1` 22, `hypot` 14,824, `log` 265,
   `log1p` 43, `log10` 3,024, `log2` 0, `pow` 2,032 (after the edge
   rules), `sin` 961, `sinh` 103, `tan` 760, `tanh` 3,539. For example
   `Math.exp(1)` is `2.7182818284590455` here and `2.718281828459045`
   under `node`. `Math.random` gives the seeded sequence of stdlib.md
   §5 rule 6; `node` gives an unseeded sequence. The other members
   agree with `node` on every measured input. An entry that prints a
   form-4 or `random` result is `js-comparable: no C4`.
10. `Number` parsing and formatting (stdlib.md §7). Divergences: a
   radix outside 2 to 36, or a digit count outside the range of the
   member, traps `invalid-argument`; `node` throws `RangeError` for
   the format members and gives NaN from `parseInt` (radix 0 is its
   default). `parseInt` gives the correctly rounded double of the
   digit prefix (it equals a BigInt reference on every row); `node`
   approximates at the radixes other than 2, 4, 8, 10, 16, and 32,
   which ECMA-262 permits. Measured over 10,100 random strings and
   radixes: 3,265 differences of 1 to 6 ulp, all at those radixes; for
   example `parseInt("+7hakjc0uq87wsasj!", 34)` is
   `7.043295590006961e+23` here and `7.043295590006962e+23` under
   `node`. `toString(radix)`, `toFixed`, `toExponential`,
   `toPrecision`, and `parseFloat` equal `node` on every row of their
   tables (10,100 to 20,200 rows each).

Accept: `a02`, `a03`, `a04`, `a10`, `a37`, `a68`, `a69`, `a70`, `a71`, `a74`, `a75`, `a76`. Reject: `r15`, `r22`, `r44`, `r57`, `r149`, `r150`, `r151`, `r156`, `r157`, `r158`,
`r58`. Trap: `t12`. Retired by P2.4: `retired:r21-string-plus-number`,
`retired:r38-string-plus-assign-number`.

## C5. Strings

Strings are the native UTF-8 Luau strings. P2.4 follows the owner's
prior project: **every length, index, and position of an accepted
string member is a byte measure.** On ASCII text the result equals the
JavaScript result. On non-ASCII text a byte measure differs from the
UTF-16 unit that JavaScript counts. This is a divergence: measured,
`"héllo".length` is 6 here and 5 in `node`; `"日本".indexOf("本")` is 3
here and 1 in `node`. An entry whose positions cross a non-ASCII
character is `js-comparable: no C5`.

1. Each string literal and each template literal part decodes to a
   sequence of Unicode scalar values. An unpaired surrogate is
   rejected in every escape form: `\uD83D`, `\u{D83D}`, in a string
   literal and in a template. UTF-8 has no form for a lone surrogate.
   The host boundary (P3) validates the UTF-8 of each string that it
   passes in.
2. Accepted operations:
   - `+` and `+=` (C4.6), template concatenation, `===` and `!==`
     (C7);
   - `s.length`: the byte length;
   - `slice(start?, end?)`, `substring(start, end?)`, `substr(start,
     length?)`, with the ECMAScript negative, clamp, and swap rules of
     each method over byte offsets;
   - `indexOf(needle, from?)`, `lastIndexOf(needle, position?)`,
     `includes(needle, from?)`, `startsWith(needle, position?)`,
     `endsWith(needle, endPosition?)`, with byte offsets and the
     ECMAScript clamps; an empty needle matches as ECMAScript says;
   - `charAt(i)`: the code point that starts at byte `i`, or `""` out
     of range; `charCodeAt(i)`: the byte value at `i`, where `i` is a
     code-point boundary (0 to 127, or a leading byte 194 to 244); `codePointAt(i)`
     and `at(i)` (a negative `i` counts from the end): the code point
     that starts at byte `i`;
   - `split(sep, limit?)` with a string separator: no match gives the
     whole string, adjacent separators give empty strings, `limit`
     follows ECMAScript ToUint32; an empty separator gives one piece
     per code point, and `"".split("")` is `[]`;
   - `trim()`, `trimStart()`, `trimEnd()`, `trimLeft()`, `trimRight()`:
     the ECMAScript WhiteSpace and LineTerminator set (U+0009 to
     U+000D, U+0020, U+00A0, U+1680, U+2000 to U+200A, U+2028, U+2029,
     U+202F, U+205F, U+3000, U+FEFF); U+0085 is not in the set;
   - `repeat(n)`; `padStart(len, pad?)`, `padEnd(len, pad?)` with byte
     lengths and `pad` `" "` by default; an empty `pad` returns the
     receiver;
   - `replace(pattern, replacement)` (the first occurrence) and
     `replaceAll(pattern, replacement)` with a string pattern: `$$`,
     `$&`, `` $` ``, and `$'` in the replacement are substituted; `$1`
     to `$9` stay literal; an empty pattern matches at the start
     (`replace`) or at every code-point boundary (`replaceAll`);
   - `concat(other)` with one string;
   - `for (const c of s)`: one code point per step, as in JavaScript.
   A number argument converts as ECMAScript ToIntegerOrInfinity
   converts it (C6.9).
3. Traps (divergences; JavaScript gives a value):
   - a position argument or a result boundary that is not on a UTF-8
     code-point boundary traps `string-boundary` (for example
     `"é".slice(1)`). A comparison window of `startsWith`, `endsWith`,
     `includes`, or a search that would end inside a code point is no
     match and does not trap (`"café".endsWith("e")` is `false`);
   - `charCodeAt`, `codePointAt`, and `at` outside the string trap
     `index-out-of-range` (JavaScript gives NaN or `undefined`);
   - `repeat` with a negative or infinite count traps
     `invalid-argument` (JavaScript throws RangeError).
   The prelude declares `at` and `codePointAt` with a `string` and a
   `number` result (D16).
4. Rejected:
   - an index `s[i]`: the element of a string is a UTF-16 unit in
     JavaScript and has no byte form;
   - `<`, `>`, `<=`, `>=` between strings;
   - `toLocaleUpperCase`, `toLocaleLowerCase`, `localeCompare`:
     the result depends on locale data;
   - `normalize`: Luau has no Unicode normalization data;
   - `match`, `matchAll`, `search`, a `RegExp` argument, and
     `String.raw`, `String` as a value;
   - `String.fromCharCode` and `String.fromCodePoint` with other than
     one argument.
5. Byte operations keep the prelude name `Utf8.byteLength`, equal to
   `length`.
6. Case and code points (stdlib.md §9): `toUpperCase()` and
   `toLowerCase()` apply the Unicode 17.0 default case conversion,
   equal to `node`. `String.fromCodePoint(cp)` and
   `String.fromCharCode(c)` take one argument. Divergence: a code point
   outside 0 to 0x10FFFF traps `invalid-argument` (`node` throws
   RangeError); a surrogate traps `invalid-argument` (`node` gives a
   lone surrogate).

Accept: `a11`, `a12`, `a19`, `a37`, `a38`, `a39`, `a80`, `a81`. Reject: `r09`, `r10`, `r34`, `r37`, `r163`, `r164`.
Trap: `t06`, `t14`, `t15`. Retired by P7.5: `retired:r18-to-upper-case` (stdlib.md §9). Retired by P2.4: `retired:r08-string-length`,
`retired:r29-string-slice`, `retired:r30-string-index-of`,
`retired:r31-string-trim`, `retired:r32-string-position-argument`,
`retired:r33-split-non-literal`, `retired:r39-string-for-of`.

## C6. Arrays

1. An index expression `xs[i]` requires `i: number` (surface.md §3) and
   lowers to a read of key `i + 1`. The
   range check tests `i` before the addition. For
   `i = 0.9999999999999999`, `i + 1` rounds to 2 and Luau reads an
   element where JavaScript reads `undefined`.
2. A read with an index that is not an integer in `[0, length)` traps
   with `index-out-of-range`. JavaScript gives `undefined`, which is
   outside the element type.
3. A write with an integer index in `[0, length]` is accepted. An index
   equal to `length` appends. Any other index traps with
   `index-out-of-range`. This is a divergence. Measured with `node`:
   an integer index above `length` makes a hole, which reads as
   `undefined`; a negative, non-integer, NaN, infinite, or very large
   index (`-1`, `0.5`, `NaN`, `Infinity`, `1e300`, `2^32`) writes a
   property that is not an element, and `length` does not change. The
   Luau run traps in each case. The check runs at the store, after the right side is
   evaluated, against the length at that time (C12).
4. `length` lowers to `#xs`. A write to `length` is rejected.
5. An element type that includes `null` is rejected. A `nil` element
   makes the Luau border ambiguous: `#{1, 2, nil}` gave 2 where
   JavaScript gives 3.
6. `for (const x of xs)` evaluates `xs` once. Before each step it reads
   the live length of that array. A later assignment to the binding
   `xs` does not change the iteration (C12).
7. The accepted members are those of rule 9 and surface.md §6. Every
   other member is rejected.
9. **Array methods (P2.3).** The set follows the owner's prior project.
   Each method lowers to a prelude helper that implements the
   ECMAScript algorithm for the accepted forms.
   - Without a callback: `indexOf(x, from?)`, `lastIndexOf(x, from?)`,
     `includes(x, from?)`, `join(sep?)`, `slice(start?, end?)`,
     `concat(other)` with exactly one array, `fill(x, start?, end?)`,
     `reverse()`, `splice(start, count?)` (delete only, returns the
     removed elements), `unshift(x)` (one element, returns the new
     length), `copyWithin(target, start, end?)`, `at(i)`, `pop()`,
     `shift()`, and the statement `xs.length = 0`.
   - With a callback: `forEach`, `map`, `filter`, `some`, `every`,
     `find`, `findLast` (stdlib.md §6), `findIndex`, `findLastIndex`,
     `flatMap` (the callback returns an array), `reduce(f, init)` and `reduceRight(f, init)` with `init`
     required, and `sort(cmp)` with the comparator required.
   - A callback takes `(v)` or `(v, i)`; the callback of `reduce` and
     `reduceRight` takes `(acc, v)` or `(acc, v, i)`; the comparator of
     `sort` takes exactly `(a, b)`. The parameter after the index, the
     array, is rejected. A one-parameter comparator is rejected: it is
     not consistent unless it is constant.
   - The element type of a result array (`map`, `flatMap`) follows
     C6.5: a callback result type that includes `null` is rejected.
   - A number argument (`from`, `start`, `end`, `count`, `target`,
     `i`) is converted as ECMAScript ToIntegerOrInfinity converts it:
     NaN is 0, a fraction truncates toward 0. A negative position is
     `length + position`, clamped as ECMAScript clamps it.
   - `indexOf` and `lastIndexOf` compare with `===`; `includes` uses
     SameValueZero and finds `NaN`. An element type of function type
     is rejected for these three (C7: closure identity).
   - `join` formats a number element as `String(n)` (C4.5), a boolean
     as `true` or `false`, and a string as itself. An element type of
     array or function type is rejected for `join`.
   - `sort` is a stable merge sort that calls `compare(earlier, later)`.
     A comparator result that is NaN counts as 0. Divergences: the
     number and the order of comparator calls are not those of `node`;
     and for a comparator that is not consistent (ECMAScript: the sort
     order is then implementation-defined), the result order is not
     that of `node`. Measured: `[3, NaN, 1, 2].sort((a, b) => a - b)`
     gives `1,2,3,NaN` here and `3,NaN,1,2` in `node`; `(x, y) => x > y ?
     1 : 0` over `[3,1,2,5,4]` gives `1,2,3,4,5` here and `3,1,2,5,4` in
     `node`. An entry with such a comparator, or with a comparator that
     has a visible effect, is `js-comparable: no C6`.
   - `pop()`, `shift()` on an empty array and `at(i)` outside the array
     trap `index-out-of-range`. Divergence: `node` gives `undefined`.
     The prelude declares their result as `T` (D16: no `undefined`).
   - A method with a callback reads the length at its start. If a
     callback changes the length of the receiver, the method traps
     `array-modified` after that callback. Divergence: `node`
     continues, skips or reads missing elements, or makes holes.
   - `xs.length = n` with `n` other than the literal `0` stays rejected:
     a larger length makes holes.
   - Rejected: a
     `sort` without a comparator (it compares strings), a `reduce`
     without `init` (it changes meaning with the argument count),
     `flat`, `entries`, `keys`, `values`, `toString`, a method read as
     a value, and every variadic form.
10. **`Map` and `Set` (P7.4, stdlib.md §8).** A key compares by
   SameValueZero for `number` (one NaN key; `-0` stored as `+0`), by
   content for `string`, by value for `boolean`, and by identity for a
   class, record, or array. Iteration follows the first insertion.
   `get` gives the absent type of C2. Divergence: a `forEach` callback
   or a `for…of` body that adds, deletes, or clears an entry of the
   container it traverses traps `map-modified`; `node` continues, visits
   an entry added during the traversal, and skips an entry deleted
   before its turn. A `set` on a present key does not trap. An `await`
   in the body of a `for…of` over a `Map` or `Set` is rejected.
8. An index signature type `{ [k: string]: T }` is rejected. A missing
   key reads `undefined` in JavaScript and `nil` in Luau, outside `T`.

Accept: `a09`, `a13`, `a32`, `a33`, `a34`, `a72`, `a77`, `a78`, `a79`, `a82`. Reject: `r06`, `r23`, `r45`, `r64`, `r159`, `r160`, `r161`, `r170`,
`r90`, `r91`, `r92`, `r93`, `r94`, `r96`. Trap: `t01`, `t02`, `t03`, `t04`, `t05`, `t07`, `t13`.
Retired by P7.2: `retired:r24-array-find` (D32).

## C7. Equality

- `===` and `!==` require operands of the same type.
- `a == b` has the type rules, the result, and the lowering of
  `a === b`. `a != b` has those of `a !== b`. With a `null` operand, C2
  applies. Measured: `tsc` gives TS2367 for `==` between `number`,
  `string`, and `boolean` in each mixed pair; over 69 same-type pairs
  `node` gives the same result for `==` and `===`.
- `===` and `!==` between function values are rejected. The Luau
  compiler reuses a closure that captures no upvalue, so two closures
  that JavaScript keeps distinct can compare equal.
- The operands of `===`, `!==`, `==`, and `!=` have one type:
  `number`, `string`, or `boolean`; or two arrays with identical element
  types, compared by identity; or a `null` operand under C2. Each other
  pair is rejected, a mixed `number` and `string` pair included.
- Two operands of one object type compare by identity, as two arrays
  do; a class instance compares by identity within its class (P2.6).
- An operand that is an array literal or an object literal (in
  parentheses or not) is rejected: the comparison is by identity, and
  `tsc` gives TS2839.
- `NaN` is an ordinary `number` operand: `NaN === x` is `false` in both
  JavaScript and Luau.
- The rule is type-based (D17). It does not depend on the shape of an
  operand or on an earlier comparison. `tsc` can still give TS2367 (the
  literal types of the operands do not overlap) or TS2845 (a comparison
  with `NaN`) at an accepted equality. CLAUDE.md invariant 3 allows
  exactly these two codes at an accepted equality expression; the
  corpus witness checks the position and the operand types of each
  (corpus.md §4). Measured: 30 programs that the earlier structural
  rule rejected run equal to `node`; 20 of them show TS2367 or TS2845
  and no other diagnostic (`tracking/equality-measurements.md`).
- Retired with the structural rule: `retired:r17-loose-equality`,
  `retired:r70-constant-equality`, `retired:r71-boolean-let-equality`,
  `retired:r72-tested-reference-equality`, `retired:r73-nan-equality`,
  `retired:r74-template-equality`, `retired:r76-test-after-block`,
  `retired:r77-test-in-for-update`, `retired:r79-aliased-condition`,
  `retired:r80-literal-condition-edge`, `retired:r83-alias-test-bound`.

Accept: `a10`, `a14`, `a28`, `a29`, `a35`, `a36`, `a41`. Reject: `r16`, `r89`, `r95`.

## C8. Forms outside the subset

`tsc` accepts each form below, and this checker rejects it. A row
states the reason. "No lowering is decided" marks a candidate to
accept later with evidence.

| # | Form | Reason |
|---|---|---|
| 1 | `any`, `unknown`, `object`, `never` in an annotation | No checked representation. |
| 2 | `var` | It binds `undefined` before its declaration (C2). |
| 3 | A `let` with no initializer | A read before assignment gives `undefined` (C2). |
| 4 | `typeof`, `in`, `delete`, `void`, the comma operator, a tagged template | No lowering is decided. |
| 5 | `for…in`, `do…while`, a labeled statement, `switch` | No lowering is decided. |
| 6 | `interface`, a generic declaration, a class with `extends` | No lowering is decided. Decided in P2. |

`erasableSyntaxOnly` in `tsconfig.json` makes `tsc` reject enums,
namespaces with values, and parameter properties. Node runs the corpus
with type stripping, which cannot run those forms. Node does not read
`tsconfig.json`.

Reject: `r01`, `r11`, `r25`, `r26`, `r27`, `r28`.

## C9. Loop bindings

1. Each iteration of a `for` loop gives each `let` binding of its head
   a new binding, with the value that the previous iteration left.
2. For a binding that a closure in the body captures, the lowering
   copies the binding into a local of the body at the start of each
   iteration and copies the local back before the update. The
   copy-back runs on every path to the update, `continue` included.
   A binding that no closure captures has no observable per-iteration
   identity, so the lowering uses the head binding directly (P6.S2).
3. A closure anywhere in a `for` head (initializer, condition, update)
   is rejected. In JavaScript a closure in the head binds to one
   specific iteration's binding, which the body-local copy of rule 2
   does not give.
4. Each iteration of `for…of` gives its `const` binding a new binding.

Accept: `a07`, `a09`, `a17`, `a65`, `a67`. Reject: `r42`, `r49`.

## C10. Declaration order

JavaScript throws `ReferenceError` for an access in the temporal dead
zone. A Luau lowering reads `nil` or a global and does not trap. This
compiler rejects each program that can reach the zone. "The end of a
declaration" is the end of the whole declaration, the initializer
included.

1. A reference to a `let` or `const` binding must come after the end of
   its declaration in source order, at any nesting depth. So a `const`
   arrow cannot call itself; a recursive function is a function
   declaration.
2. A reference to a `class` binding must come after the end of its
   declaration, or inside the body of that class. A method runs only
   after the class exists.
3. A function declaration is hoisted. Let F be a function declaration,
   and let B be a `let`, `const`, or `class` binding declared outside
   F. F reaches B if F refers to B, or if F refers to a function
   declaration that reaches B. A reference inside a function or arrow
   that F creates, at any depth, is a reference of F: the closure can
   run while B is in its zone (`function mk() { return () => x; }`
   called before `let x`). The relation is transitive over chains
   of any length. If F reaches B, every reference to F must come after
   the end of the declaration of B in source order. A reference is a
   call or a use as a value, at any nesting depth.

Reject: `r20`, `r35`, `r36`, `r40`, `r41`, `r50`, `r51`, `r56`, `r86`.

## C11. The accepted surface is closed

`surface.md` lists every accepted form. Every other syntax form, type
form, global, namespace, and member is rejected.

The source is an ECMAScript module, so it is strict-mode code. Each
early error of strict-mode module code gives C11: a legacy octal
literal (`010`, `08`), a legacy octal or `\8`, `\9` escape, and a
binding named `eval`, `arguments`, `await`, or a strict reserved word
(`implements`, `interface`, `let`, `package`, `private`, `protected`,
`public`, `static`, `yield`). Without the rule, `node` does not run
the program. `tsc` gives TS1121, TS1214, TS1215, TS1487, TS1488, or
TS1489; for `await` it gives TS1262 at module level and no error in a
function body, and `node` gives a SyntaxError in both places. A rule in this file
explains a listed form or a rejection near the list. It does not make
a form acceptable.

Accept: `a27`, `a30`. Reject: `r47`, `r55`, `r59`, `r60`, `r61`, `r62`, `r65`, `r66`,
`r75`, `r78`, `r81`, `r82`, `r85`, `r87`, `r88`, `r98`, `r99`, `r100`, `r104`, `r105`, `r106`, `r107`, `r112`, `r116`, `r117`.
Retired: `retired:r97-narrowed-let-copy` (D18 revision), `retired:r46-math-round` (P7.1: `Math` is accepted, stdlib.md §5).

## C12. Evaluation order

JavaScript evaluates each expression from left to right, and reads a
binding or a field at a fixed point of that order. A Luau operator or
call can read a local, a field, or a table later: `n + inc()` reads `n`
after `inc()` runs. The rule:

**The lowering never reads a binding, a field, an element, or an array
length at a different point of the evaluation than JavaScript does.**
Where the plain Luau form reads a value later, the lowering copies the
value into a temporary at the JavaScript point. Where the plain Luau form
evaluates a part earlier, the lowering orders the statements.

Instances of the rule, each pinned in `a18`; P2.2 also pins 3, 4, 5,
and 6 in `a31`:

1. A binary operator reads its left operand before it evaluates the
   right operand.
2. A call reads its callee, then the receiver of a method call, then
   the arguments from left to right.
3. An index read `xs[i]` reads `xs`, then evaluates `i`, then reads the
   element.
4. An assignment `o.f = e` or `xs[i] = e` evaluates `o` (or `xs` and
   `i`), then `e`, then stores. The C6 check runs at the store.
5. A compound assignment `t op= e` evaluates the target reference,
   reads the old value, evaluates `e`, then stores.
6. `xs.push(e)` evaluates `xs`, then `e`, then reads the length of
   `xs`, then stores. `xs[#xs + 1] = e` reads the length before `e`.
7. An element read `ys[j]` that is the object of a further access
   (`ys[j].f`, `ys[j][k]`, `ys[j].f(a)`, or the target of a store; not
   `?.`) takes its value without a check. When `j` is out of range,
   the trap `index-out-of-range` runs where `node` throws a `TypeError`
   on the `undefined` value: for a read, after the key `k` is evaluated
   (a compound assignment reads the old value before its right side);
   for a call, before the arguments; for a store, after the right
   side. Measured: `xs[0].f = h()` on an empty `xs` prints the output
   of `h` before the trap in both; `zs[i(0)][i(1)].f = h()` with
   `zs = []` evaluates `i(0)` and `i(1)` and throws before `h`.

Accept: `a18`, `a31`, `a50`, `a66`. Trap: `t07`.

## C13. Construction

1. A class field has a type annotation. It has an initializer, or the
   constructor assigns it.
2. A field initializer does not use `this`.
3. In a constructor, until every field without an initializer has an
   assignment on every path, `this` appears only as the target
   `this.f = e`, and `e` does not use `this`. JavaScript reads
   `undefined` from an unassigned field, outside its type. An arrow
   that reads `this` is a use of `this` where the arrow is written.
4. `new C(a, b)` evaluates the arguments from left to right, then the
   field initializers in declaration order, then the constructor body,
   as JavaScript does.
5. `this` in a method or a constructor is the receiver. An arrow at
   any depth inside a method or a constructor reads the receiver of
   that call; the arrow captures it as it captures a `const` binding.
   A nested function declaration does not see `this` (`tsc` gives
   TS2683). `this` is a value of the class type: a method can return
   it, store it, and pass it.

Accept: `a16`, `a44`, `a45`. Reject: `r52`, `r114`.

## C14. No metatables; literal values

1. The lowering sets no metatable on a script value. An object value and
   a class instance are plain tables of their fields. A `null` field is
   an absent key, and its read gives `nil` without a fallback lookup. A
   member name, such as `new` or `__eq`, is only a key; it never
   selects Luau behavior.
2. A method call is a direct call of a generated function, with the
   receiver as the first argument. Classes have no inheritance and no
   relation to object types (C1.5), and a method is never assigned
   (surface.md §3), so each method call has one target that the
   checker knows.
3. The emitter writes a number literal from its binary64 value, and a
   string literal from its decoded scalar values. It never copies the
   source spelling. Luau saturates `0xffffffffffffffffff` to 2^64 and
   rejects `0o17`; Luau reads `"a\<newline>b"` as `"a\nb"`. In a
   template literal, JavaScript reads a raw CR LF and a raw CR as LF;
   the decoded value carries LF.
4. A string literal is emitted as a quoted Luau string. Each byte
   outside `0x20`–`0x7E`, and each `"` and `\`, is a decimal escape
   `\ddd`. Luau rejects a raw CR or NUL inside quotes, and a long
   bracket string changes `\r\n` to `\n`.

5. Every emitted identifier is generated, in a namespace that no Luau
   global and no library name uses. A script binding named `math`,
   `table`, `string`, or `tostring` cannot capture a library reference
   that a lowering emits, such as `math.fmod` (C4.3). The lowering
   reads each library function through a name that only the compiler
   writes. The rule includes a script name that is a Luau keyword
   (`end`, `local`, `nil`, `then`) or a Luau global (`error`, `pcall`).

Accept: `a20`, `a21`, `a24`, `a25`, `a26`.

## C15. Luau compiler limits

The Luau compiler and VM have fixed limits, for example 200 locals and
200 upvalues per function, 255 registers, a parser recursion limit of
1000, and 2^15 closures per function. A program that `tsc` accepts can
exceed each one: 201 module-level `const` bindings with non-constant
initializers gave `Out of local registers`. A failure to compile an accepted program is neither the
`node` result nor a trap.

**Rule: the compiler keeps every accepted program inside every fixed
Luau limit.** It uses two means:

1. **Lowering forms.** For every input inside the bounds of rule 2, each
   generated Luau function stays inside the local, upvalue, and
   register limits. Locals include parameters, temporaries (C12), loop
   copies (C9.2), and any value that the lowering adds to reach spilled
   bindings. Values beyond a budget move to tables. The P1 pipeline
   contract defines the form, including nested scopes and per-iteration
   tables. A scope that needs no table creates none.
2. **Compiler bounds.** Where no lowering form helps, the compiler sets
   a bound and rejects an input over it with a limit diagnostic that
   names the bound. The bounds cover at least: source size, nesting
   depth counted over expression and statement terms of the typed IR
   (each operator of a flat chain counts), parameters per function,
   arguments per call, operands of one concatenation, and closures per
   function where the source ceiling does not already bound them
   (`pipeline.md` §5.14.7: it does for closures). The register budget of an expression is the register
   limit minus the locals of rule 1.

P1 measures each bound against the pinned Luau and records its value
in a contract. A gate test builds, for each bound, an input at the
bound, which must compile, and an input one step past it, which must
give the limit diagnostic. Further gate inputs combine the bounds: the
local budget together with nested scopes that each hold bindings, the
maximum captures, and the maximum depth. A Luau compile error on
generated source is a compiler defect in every case.

A limit diagnostic and a resource-limit trap are outside the `node`
comparison (`CLAUDE.md` invariants 2 and 5). The cost of a table read
is visible in the source: it follows from the number of bindings that
the source declares (principle 16).

Accept: `a22`, `a23`.

## C16. Narrowing

`tsc` narrows `T | null` to `T` after a null test, and keeps the narrowed
type across calls and for fields. A call can assign `null` to the
location in between, and JavaScript then reads `null` where the type
says `T`. The checker narrows only where nothing else can run a write:

1. A null test (`x != null`, `x == null`, `x !== null`, `x === null`)
   narrows a reference `x` on the edges where the test holds. The edges
   compose through `!`, `&&`, `||`, and parentheses (rule 9), and reach
   the branches of `if`, `?:`, `&&`, and `||`, the body and the exit of
   a loop, and the statements after an early `return`, `break`, or
   `continue`. A `?:` used as a test is a leaf of rule 9: it narrows
   nothing on its edges (`if (c ? x != null : false)` gives TS18047 in
   `tsc`).
2. `x` is a `const` binding, or a `let` binding or a parameter that no
   function or arrow assigns. A module-level `let` is never narrowed.
3. The narrowing ends at the next assignment to `x` on that path.
4. A null test inside a nested function or arrow body narrows a `let`
   binding or a parameter of an enclosing function, within that body,
   when rule 2 holds for it: the enclosing activation does not run
   between the test and the read. An `await` in the nested body
   suspends it, and the enclosing activation can run and assign the
   binding: each `await` of the nested body ends the narrowings of the
   `let` bindings and the parameters of the enclosing functions on its
   path (C19). The check is conservative: an `await` in a `finally`
   block also ends them on each jump into the join of an enclosing
   loop, also a jump that does not pass the `await`. The narrowing of a `let` binding or
   a parameter does not continue
   into a nested function or arrow body. The body can run after a later
   assignment. The narrowing of a `const` binding continues into an
   arrow body. It does not continue into a nested function declaration:
   the declaration is hoisted, and `tsc` gives TS18047 there.
5. A field, an element, and every other reference are never narrowed.
   Copy the value into a `const` to narrow it.
6. A point that two or more edges reach has the narrowings that every
   edge carries. The update of a `for` is such a point: its edges are
   the end of the body and each `continue` of the loop.
7. The narrowing costs O(1) amortized per syntax node and per edge
   (`pipeline.md` §3.10). The compiler computes the narrowings of each
   condition node once, at any depth of `?:`, `&&`, and `||`. An edge
   into a join (a `break`, a `continue`, the end of a branch or a body)
   costs O(1) amortized per narrowing change since the previous edge
   into that join. It does not walk the path from the loop entry or
   the branch start again.
8. At a point that no edge reaches, no reference is narrowed: a read
   there has the declared type. Such points are the statements after
   a `return`, `break`, or `continue`, after an `if` whose branches all
   exit, after a loop that never ends normally, and a `for` update that
   no edge reaches. `tsc` gives a read there the declared type
   (TS18047). An arrow function at such a point is rejected (C16).
   `tsc` checks the body of an arrow that is called where it appears
   as a part of the enclosing flow, so it narrows nothing in that body
   (TS18046 for a catch binding, TS18047 for a nullable value).
9. Only these end an edge: `return`, `throw`, `break`, `continue`, a loop
   that never ends normally, and a literal leaf of a test. A test is
   *logical* when, without its parentheses and `!` operators, it is an
   `&&` or `||` expression. A logical test composes its edges from its
   operands: `!` swaps the true and false edges, parentheses keep them,
   `a && b` and `a || b` join the edges of `a` and `b` as their
   evaluation order gives. Every other test is a leaf. A leaf `true`,
   with no parentheses and no `!`, has no false edge; a leaf `false`
   has no true edge; every other leaf has both edges. So `true || c`,
   `(true || c)`, and `!(false && c)` have no false edge; `(true)`,
   `!false`, `!!true`, `(true) || c`, and `c ? true : true` have both.
   A null test never ends an edge, also when it contradicts a
   narrowing: `tsc` keeps that edge and narrows the reference to
   `never` there. On that edge the other narrowings stay, and the
   reference has no fact of its own: at a join it keeps the narrowing
   of the other edges (`x != null || x != null` narrows `x`). The edge
   does not occur at run time, because the narrowed reference does not
   change between the two tests (rules 2 and 3).
   `tsc` composes `a ?? b` as `a || b`, and a literal that is an
   operand of `??` keeps both edges. JavaScript also reaches the false
   edge when `a` is `false`. So a `??` of type `boolean` whose right
   operand, composed by this rule, lacks a true or a false edge is
   rejected (C16): `bn ?? (true || c)` is rejected, `bn ?? true` is
   accepted. Ulua narrows nothing through a `??` test. A loop never
   ends normally when its test is absent or has no false edge, and no
   `break` of the loop is reachable. These are the `tsc` edges,
   measured.

Accept: `a14`, `a42`, `a43`, `a46`, `a47`, `a48`. Reject: `r67`, `r68`, `r69`, `r101`, `r103`, `r108`, `r109`, `r110`, `r111`, `r115`, `r124`, `r125`, `r142`.

## C17. Exceptions

JavaScript throws any value and catches it with `try`. Luau raises an
error with `error` and catches it with `pcall`, and `pcall` also
catches the errors that stop a run: traps, `limit:memory`,
`limit:steps`, `limit:stack`, and host errors. Measured: inside
`pcall`, each of these returns `false` with a string, and the current
shim then reports success if the generated code drops the error. A
message comparison cannot separate them: `error("not enough memory",
0)` gives the same string as an allocation failure. The measurements
are in `tracking/p2-values.md` (P2.7 Step 0).

1. **The Error family.** `Error`, `TypeError`, `RangeError`,
   `SyntaxError`, `ReferenceError`, `EvalError`, and `URIError` are
   classes of the prelude (`lib.es5.d.ts`). A value of the family has
   two fields, `name: string` and `message: string`, and is a plain
   table of them (C1, C14.1). `new C()` sets `message` to `""`;
   `new C(m)` sets it to `m`; each sets `name` to the class name. A
   second constructor argument, a call without `new`, and every other
   member (`stack`, `cause`) are rejected (C17). The seven names denote
   one static type: a value of one class flows to a binding of
   another, as `tsc` relates them structurally.
2. **Identity.** `x instanceof C`, with `C` a class of the family,
   tests the class that constructed `x`, not its `name` field (a
   program can assign `name`). Every family value is an instance of
   `Error`. The lowering keeps the class in a private table with weak
   keys that the prelude owns; it adds no field and no metatable to the
   value (invariant 4). Every other `instanceof` is rejected (C11).
3. **`throw e`.** `e` has the family type. `throw 42` and `throw "x"`
   are `tsc`-clean and are rejected. `throw` ends its edge, as
   `return` does (C16.9).
4. **`try`.** `try { } catch (e) { }`, `catch { }`, and `finally { }`
   are accepted in the three JavaScript combinations. `catch (e:
   unknown)` is `catch (e)`. Every other annotation is rejected
   (`tsc` gives TS1196 for `Error`; `any` is outside the subset). The
   catch binding is a `const`: its two uses are `e instanceof C` and
   `throw e`. In the true branch of `e instanceof C`, `e` has the
   family type (C16 rules apply to the test). `e.message` without the
   test is rejected (`tsc` TS18046).
5. **Propagation.** An exception leaves the nearest enclosing `try`
   block of the raise point through every call form: direct calls,
   methods, arrows, function values, and the callbacks of array
   methods and of `sort`. Measured: `node` delivers the same object
   from each of these.
6. **`finally`.** The finalizer runs once on each exit of the `try`
   block and of the `catch` block: fall-through, `return`, `break`,
   `continue`, and an exception. When the finalizer falls through, the
   earlier completion continues: the return value evaluated before the
   finalizer, the same exception object, or the same jump. A `return`,
   `throw`, `break`, or `continue` in the finalizer, and an exception
   that leaves it, replace the earlier completion. Measured in `node`:
   4 pending completions by 5 final completions, 20 cases.
7. **Stops are not exceptions.** A trap, `limit:memory`,
   `limit:steps`, `limit:stack`, and a host error stop the run. No
   `catch` receives them and no `finally` runs. A script raises only
   family values (rule 3), and the prelude table of rule 2 holds each
   of them. So a value that a protected block receives and that the
   table does not hold is a stop: the generated code raises it again
   with `error(v, 0)`, before any `catch` or `finally` code, without
   a change and without an allocation (measured: zero allocator
   requests), and the run reports the code that the stop has without a
   `try` (`pipeline.md` §4). A message comparison does not decide it:
   a script cannot raise a string. Divergence: `node` throws a
   `RangeError` or a `TypeError` that a `catch` receives where this
   run traps (`"x".repeat(-1)`, `invalid-argument`). An entry with
   such a `try` is `js-comparable: no C17`.
8. **An uncaught exception.** An exception that leaves an exported
   function stops the run with `ULUA_FAILED` and the code
   `trap:uncaught-exception`. The diagnostic message is the `name`,
   `": "`, and the `message` of the value. `node` prints the same
   value and exits with status 1.
9. **Cost and limits.** Each `try`, `catch`, and `finally` block that
   runs allocates one closure: a cost that the source shows (principle
   16). A `break` or `continue` in a `try` or `catch` block that leaves
   the statement to an outer loop is a jump of the statement: the
   statement completes with that jump, also when the block ends with a
   `return`. The time of `luau-analyze -j1` over the output of `try`
   shapes grows at most 12 times for 4 times the input (`pipeline.md`
   §4.9). A `try`,
   a `catch`, and a `finally` block each count as a nesting term of
   `limit:nesting-depth` and add their generated closure to the Luau
   nesting. Measured: `luau-analyze` reports "Code is too complex" for
   75 nested protected closures in one function and passes at 70; Luau
   gives "C stack overflow" at 198 nested protected calls
   (`limit:stack`).
10. **Narrowing.** At the entry of a `catch` block, a binding that the
   `try` block assigns has no narrowing; the other narrowings are those
   at the entry of the `try` statement. At the entry of a `finally`
   block, the same holds for the bindings that the `try` block or the
   `catch` block assigns.

Accept: `a51`, `a52`, `a53`, `a54`. Reject: `r119`, `r120`, `r121`, `r122`,
`r123`. Trap: `t08`, `t09`.

## C18. Host functions

JavaScript has no host functions: an embedder installs native
functions as global values, which a script can read, copy, compare,
and replace. In Ulua a host function is a capability that the host
grants for one compilation (`abi.md` §6.2, D20, D21). The grant list
is the only record of it: the library writes its declarations for
`tsc` (`abi.md` §6.3), and the script declares nothing. A capability
is not a value: a script can only call it. A value use would let a
reference leave the grant check, and Luau compares functions by
identity where JavaScript compares the global binding.

1. **Grants.** Each grant of `ulua_compile` is a host function of the
   script: a name in the outermost scope, outside the module scope,
   with the parameter types and the result type of the capability.
   `tsc` sees the same name through the declarations of `abi.md` §6.3,
   which are global.
2. **No declarations.** Every `declare` form of the script
   (`declare function`, `declare const`, `declare let`, `declare
   class`, `declare namespace`, `declare module`, `declare global`)
   is rejected (C18).
3. **Unknown names.** A reference to a name that no binding of the
   script, no prelude or library name, and no grant declares is
   rejected (C18). `tsc` gives TS2304 for it. A library name that the
   surface does not accept (`Number`, `JSON`, `parseInt`), a type-only
   library name in a value position (`Iterator`, `Record`; `tsc`
   TS2693), and the implicit `arguments` object give C11, as before
   P3. A `namespace`, `module`, or `global` block without `declare`
   gives C11.
4. **Calls only.** A reference to a host function is the callee of a
   `CallExpression`, without `?.`, in any position where a call is
   accepted, also in an arrow, a method, a `try` block, and a
   default. Every other use is rejected (C18): a value read, an
   assignment, a comparison, an argument, an export, a field value,
   `typeof`.
5. **Scopes.** A script binding of the same name shadows a host
   function in the scope of the binding, as a module binding shadows a
   global in JavaScript. A grant that the script shadows everywhere is
   unused.
6. **Evaluation.** A call evaluates its arguments left to right
   (C12), then calls the host once. The result has the grant's type.
   A host error or a revoked grant stops the run (`abi.md` §6.2,
   D22): it is a stop under C17.7, never an exception. A host function
   has no temporal dead zone (C10 does not apply).
7. **Agreement.** The corpus grants a fixed set of fixture host
   functions (corpus.md §7). The native harness and the `node` runner
   implement each fixture with the same result for the same
   arguments, so an entry that calls fixtures is `js-comparable`.

Accept: `a55`, `a56`, `a57`. Reject: `r126`, `r128`, `r130`, `r131`,
`r132`, `r133`. Retired by the D20 revision:
`retired:r127-host-signature` (a grant gives the signature),
`retired:r129-host-shadow` (rule 5 accepts it, `a57`).

## C19. Async functions and promises

JavaScript runs an `async` body until its first `await`, returns a
promise, and runs each continuation as a job of one FIFO queue. Luau
has coroutines and no promise. Ulua keeps the job model of ES2022 for
native promises (D24, D25): a promise is a prelude table, each
`async` activation is a coroutine, and the jobs run in the order that
`node` runs them. Measured in `tracking/p4-async.md` (P4 Step 0): a
Luau model of the jobs gives the `node` order on 2,040 programs.

1. **Forms.** An `async` function declaration, method (not a
   constructor), or arrow has the result type `Promise<T>`, written.
   `T` is an accepted type, or `void`. Generators, `async`
   generators, `for await`, and `await using` are rejected (C19).
2. **The promise type.** `Promise<T>` is a type of values: a binding,
   a parameter, a field, an array element, a result. `T` is not a
   `Promise` type. `T` has no member named `then` (a field of a record
   type, or a field or a method of a class): JavaScript adopts such a
   value as a thenable and calls its `then`, and `tsc` types the
   `await` as `Awaited<T>`. The job model holds native promises only. A promise relates only to an identical promise type
   (C1): `Promise<number>` does not flow to `Promise<number | null>`,
   which `tsc` accepts. A promise value has no member and no operator
   other than those of rules 10 to 13: an equality, a template substitution, a truth test,
   `typeof`, and an array search (`indexOf`, `lastIndexOf`,
   `includes`) over promise elements are rejected (C19). The `Promise`
   identifier is accepted only as the object of `Promise.all` (rule
   13); as a value (`new Promise`, `Promise.resolve`) it is rejected
   (C19). A script declaration named `Promise` (a class, a type alias)
   is rejected (C19): `tsc` then resolves `Promise<T>` to it (TS2315).
3. **`await`.** `await e` is accepted only in an `async` body, not in
   a nested non-`async` function, not in a parameter default (`tsc`
   TS2524), and not at module level (`tsc`
   accepts top-level `await` in a module). `e` has a `Promise<T>`
   type; `await` of another type is rejected (C19), where `tsc`
   accepts it. The result has type `T`; an `await` of
   `Promise<void>` is an `ExpressionStatement`.
4. **No floating promise.** An `ExpressionStatement` whose value has
   a type that holds a promise value (`Promise<T>`, `Promise<T>[]`, a
   record or a class with a promise field, at any depth) is rejected
   (C19). A function type and a method hold no promise value: a
   statement `new Game();` whose class has an `async` method, and a
   statement whose value is a function that gives a promise, are
   accepted: the script stores, passes,
   returns, or awaits each promise. `tsc` accepts it.
5. **Start and settle.** A call of an `async` function runs its body
   at once, up to its first `await` or its end, and gives its
   promise. `return v` fulfills the promise with `v`; `return p` with
   a promise `p` adopts `p` with the jobs of ES2022 (a
   PromiseResolveThenableJob). An exception that leaves the body
   rejects the promise (D26). `await p` suspends the activation and
   resumes it in a job after `p` settles: with the value, or by a
   throw of the reason, which `try`/`catch` catches (C17). A stop is
   never a rejection (C17.7): it leaves the run before any `catch`,
   `finally`, or job.
6. **Order.** The jobs run in one FIFO queue in the order of ES2022:
   a reaction to a settled promise enters the queue at once; a
   reaction to a pending promise enters it when the promise settles.
   The queue drains after each host-started run (`ulua_instance_new`,
   `ulua_call`, `ulua_call_start`, and each settle of `ulua_pump`),
   before the run returns (D25).
7. **Unhandled rejection.** At the end of a drain, a rejected promise
   that has no reaction stops the run: `trap:uncaught-exception` with
   the message `name: message` of the first such promise in rejection
   order (corpus.md §6). For an `async` export whose own promise is
   rejected, the message is the reason of that promise: the host is its
   awaiter, as the `node` runner awaits `main`. `node` reports it after the same drain
   (`unhandledRejection`) and ends the process. A reaction added in the
   same drain handles it. The instance stays usable (C17.8).
   Divergence: an unhandled rejection that the module-level statements
   leave stops `ulua_instance_new` at the end of its drain, before any
   export runs. The `node` runner evaluates the module and calls `main`
   in the same job checkpoint, so `node` runs `main` first and then
   reports the rejection. Measured (`t11`): Ulua writes nothing and
   stops with `trap:uncaught-exception` `Error: module`; `node` writes
   `main`, then reports `Error: module`.
8. **Array callbacks.** An argument of an array method other than
   `map` (rule 14) whose result type is a `Promise` type (an `async`
   arrow, or a function value) is rejected (C19): `forEach` drops the promise of its callback.
   A promise that an `async` body returns to itself rejects it with a
   `TypeError`, as ES2022 does.
9. **Limits.** Each `async` activation is one coroutine. An `async`
   call that runs before the first `await` nests one coroutine resume
   in the caller; nested resumes past the Luau C-stack limit give
   `limit:stack`. Coroutines, promises, and jobs count toward
   `max_memory_bytes`, and their work toward `max_steps` (`pipeline.md`
   §4.11).

10. **`then`.** `p.then(f)` with `p: Promise<T>` and one argument:
   an arrow or a function value with one parameter of type `T` or no
   parameter, and a result type `U` or `Promise<U>` (`U` as in rule 1;
   `void` gives `Promise<void>`). The result is `Promise<U>`; a result
   `Promise<U>` of `f` is adopted with the jobs of ES2022. A second
   argument is rejected (C19): `catch` handles a rejection.
11. **`catch`.** `p.catch(f)` with one argument: an arrow or a function
   value with no parameter, or one parameter annotated `unknown`, and a
   result type `T` or `Promise<T>`. The parameter has the uses of a
   catch binding (C17.4): `instanceof` and `throw`. `tsc` types an
   unannotated parameter `any`, so the annotation is required. The
   result is `Promise<T>`.
12. **`finally`.** `p.finally(f)` with one argument: an arrow or a
   function value with no parameter and a result type `void` or
   `Promise<void>`. The result is `Promise<T>`. The jobs follow ES2022
   27.2.5.3 (Step 0 M4).
13. **`Promise.all`.** `Promise.all(xs)` with `xs` an array of
   `Promise<T>`, an array literal of one promise type, or an array
   value. `T` is not `T | null` and not `void`: `node` gives an
   element for each, and a plain Luau array cannot hold `nil` (P4
   Step 0 M4; C6.5). The result is `Promise<T[]>`; it rejects with the
   first rejection in settle order. Every other static member of
   `Promise` stays rejected (rule 2).
14. **`map`.** An `async` arrow or a function value with a `Promise<U>`
   result is accepted as the callback of `map`, which gives
   `Promise<U>[]`. Every other array method keeps rule 8.
15. A promise that `then`, `catch`, `finally`, or `Promise.all` gives
   follows rules 2, 4, 6, and 7: it is not floating, it has no
   operator, its jobs run in the queue, and its unhandled rejection
   stops the run.
16. **Async host functions.** A grant of an async capability
   (`abi.md` §6.4) is a host function with the result type
   `Promise<R>`. A call starts a request and gives its promise; the
   host settles it later through `ulua_pump`, one host-started run per
   settle, so the jobs after it run before the next settle. A failed
   request stops the run (D26): no `catch` runs. Every rule of this
   section applies to the promise.

Accept: `a58`, `a59`, `a60`, `a61`, `a62`, `a63`, `a64`. Reject: `r134`, `r135`, `r136`,
`r137`, `r138`, `r139`, `r140`, `r141`, `r143`, `r144`, `r145`, `r146`, `r147`, `r148`. Trap: `t10`, `t11`.

## C20. JSON

`JSON.stringify` and `JSON.parse` follow stdlib.md §10.

1. `JSON.stringify(value)` takes one argument of a serializable type
   (stdlib.md §10 rule 1). A function type, `Map`, `Set`, a promise, an
   Error-family value, the absent type, a type that contains one of
   them, and a `replacer` or `space` argument are rejected: `node`
   writes `{}` for a `Map` and a `Set` and omits a function field.
2. A record or a class writes its fields in declaration order. An
   object literal that reaches a `JSON.stringify` call lists its
   fields in that order; another order is rejected (D34). Divergence:
   a value from `JSON.parse` writes in declaration order; `node` writes
   the key order of the parsed text.
3. Divergence: NaN and the infinities trap `json-number`; `node`
   writes `null`.
4. A cycle throws `TypeError`, as `node` does.
5. `JSON.parse(text)` needs a contextual target type; a call without
   one is rejected. Malformed text throws `SyntaxError`, as `node`
   does. Divergence: well-formed text that does not match the target
   throws `TypeError`, a number outside the finite range throws
   `TypeError`, a lone surrogate escape throws `SyntaxError`, and text
   nested deeper than 128 throws `SyntaxError`; `node` returns the
   value (measured: 200 levels give the value in `node`).
6. Divergence: a field that the target does not declare is dropped;
   a later `stringify` omits it, and `node` keeps it (measured:
   `{"x":1}` here, `{"x":1,"y":2}` in `node`).

Accept: `a83`, `a84`, `a85`, `a86`. Reject: `r165`, `r166`, `r167`, `r168`, `r169`. Trap: `t16`.

## Open items

- Normalization of strings waits for a native helper (C5.4).
