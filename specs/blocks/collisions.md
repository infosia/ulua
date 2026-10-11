# Semantic collisions — decided rules

Status: Rev 16, 2026-10-11 (Rev 0 to Rev 11 the same day; P0 Phase
Review rounds 1 to 12 changed C1 to C6, C9, C10, C12, C14, C16 and added C11
to C16; Rev 13: C7 literal operands, P1 Phase Review round 3; Rev 14: C7
structural operand rule, round 4; Rev 15: C7 accepts `==` and `!=`;
Rev 16: the three resolutions and the divergence format, invariant 5).

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
4. Function types relate only when their parameter types are identical
   and their results relate.
5. An object type and a class never relate, in either direction, until
   P2 decides structural or nominal relation. A call of a field of
   function type reads the field and calls it. A method call is a
   direct call of the method's generated function with the receiver as
   its first argument (C14). A class instance seen through an object
   type, or an object literal seen through a class type, selects the
   wrong call form.

Accept: `a01`, `a05`, `a09`, `a15`, `a16`. Reject: `r19`, `r43`, `r53`,
`r54`.

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
- A null test is `x == null` or `x != null`. `x === null` and
  `x !== null` are rejected.
  - Reason: JavaScript gives `undefined` in places where this language
    has `null`. `==` agrees for both values. `===` does not.
- `a ?? b` is accepted when `a` has type `T | null`. It lowers to an
  if-expression on `a ~= nil` with `a` evaluated once, never to
  `a or b`: `false ?? true` is `false` in JavaScript, and `false or
  true` is `true` in Luau.
- An optional chain has exactly one optional step and no step after it:
  `a?.f` or `a?.m(args)` (surface.md §3). It is accepted in two
  positions only: `a?.f` as the left operand of `??`, and `a?.m(args)`
  as an expression statement. In any other position its type includes
  `undefined`. A step after the optional step, as in `a?.f.g`, is
  rejected: JavaScript skips the whole chain when `a` is `null`.

Accept: `a08`, `a14`, `a15`. Reject: `r02`, `r03`, `r04`, `r05`,
`r12`, `r13`, `r14`, `r48`, `r63`.

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
4. `**` and `Math.pow` are rejected. Luau `^` follows C `pow` and gives
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
6. `+` and `+=` with one `string` operand and one non-`string` operand
   are rejected. A template literal states the conversion.
7. The bitwise operators are rejected. No lowering is decided.
8. A template substitution has type `number`, `string`, or `boolean`.
   `String(x)` takes a `number`. JavaScript formats `null` as `null`,
   an object as `[object Object]`, and an array by its elements. Luau
   `tostring` gives `nil` and a table address.

Accept: `a02`, `a03`, `a04`, `a10`. Reject: `r15`, `r21`, `r22`,
`r38`, `r44`, `r57`, `r58`.

## C5. Strings

Strings are the native UTF-8 Luau strings.

1. Each string literal and each template literal part decodes to a
   sequence of Unicode scalar values. An unpaired surrogate is
   rejected in every escape form: `\uD83D`, `\u{D83D}`, in a string
   literal and in a template. UTF-8 has no form for a lone surrogate. The host boundary (P3)
   validates the UTF-8 of each string that it passes in.
2. Accepted string operations, and no others:
   - `+` between two strings, and template concatenation;
   - `===` and `!==` between two strings;
   - `includes`, `startsWith`, and `endsWith` with exactly one argument;
   - `split` with exactly one argument, a non-empty string literal.

   Each lowering agrees with JavaScript for an empty needle:
   `includes("")`, `startsWith("")`, and `endsWith("")` are `true`.
3. Every other `String` member and every other string operator is
   rejected. These are rejected because the result depends on the
   encoding:
   - `length`, an index `s[i]`, `at`, `charAt`, `charCodeAt`,
     `codePointAt`, `slice`, `substring`, `indexOf`, `lastIndexOf`;
   - the position argument of `includes`, `startsWith`, and `endsWith`;
   - `split` with a separator that is not a literal: an empty
     separator splits by UTF-16 unit in JavaScript and by byte in Luau;
   - `<`, `>`, `<=`, `>=` between strings;
   - `for…of` over a string: JavaScript steps by code point.
4. Byte operations use prelude names that JavaScript does not define:
   `Utf8.byteLength`. Further names are decided in the P2 string slice.
5. `toUpperCase`, `toLowerCase`, `trim`, `trimStart`, and `trimEnd` are
   rejected until a lowering agrees with JavaScript on non-ASCII input,
   or a revision names the divergence.
   Luau `string.upper`, `string.lower`, and `%s` act on ASCII only.

Accept: `a11`, `a12`, `a19`. Reject: `r08`, `r09`, `r10`, `r18`, `r29`,
`r30`, `r31`, `r32`, `r33`, `r34`, `r37`, `r39`.

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
   `index-out-of-range`. JavaScript makes a hole, which reads as
   `undefined`. The check runs at the store, after the right side is
   evaluated, against the length at that time (C12).
4. `length` lowers to `#xs`. A write to `length` is rejected.
5. An element type that includes `null` is rejected. A `nil` element
   makes the Luau border ambiguous: `#{1, 2, nil}` gave 2 where
   JavaScript gives 3.
6. `for (const x of xs)` evaluates `xs` once. Before each step it reads
   the live length of that array. A later assignment to the binding
   `xs` does not change the iteration (C12).
7. The accepted members are `length` and `push` (surface.md §6). Every
   other member is rejected until the P2 array slice. A member whose
   TypeScript result includes `undefined`, such as `pop` and `find`,
   stays rejected then too.
8. An index signature type `{ [k: string]: T }` is rejected. A missing
   key reads `undefined` in JavaScript and `nil` in Luau, outside `T`.

Accept: `a09`, `a13`. Reject: `r06`, `r23`, `r24`, `r45`, `r64`. Trap:
`t01`, `t02`, `t03`.

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
- `tsc` gives TS2367 when the types of the two operands do not overlap.
  A literal type, a union of literal types, or a reference that `tsc`
  narrows to one makes this possible. The checker does not model these
  types. It accepts only operand pairs where `tsc` cannot give a literal
  type to both. The rules below are restrictive on purpose; a later
  revision opens a form when a program needs it.
  1. `===`, `!==`, `==`, and `!=` do not take `boolean` operands. Write `!`, `&&`,
     `||`, or `?:`.
  2. An operand that is `NaN` (in parentheses or not) is rejected: `tsc`
     gives TS2845.
  3. At least one operand is *wide*. A wide operand is one of these
     forms:
     - a call to a function that declares its result type;
     - a binary `+`, `-`, `*`, `/`, or `%` expression;
     - `-e`, where `e` is not a numeric literal;
     - a reference to a `let` binding or a parameter that is not
       *tested* at that point;
     - a reference to a `const` binding that is not *tested* at that
       point, if its initializer is wide;
     - `(e)`, where `e` is wide.
     Every other form is not wide: literals, `-` of a numeric literal,
     templates, `?:`, `&&`, `||`, `!`, and tested references.
  4. A reference `x` is *tested* in a region where `tsc` can narrow it
     by an equality. For a condition `g`, *T*(g) is the set of
     references that `g` true can narrow, and *F*(g) is the set for `g`
     false:
     - `x === e`, `e === x`, `x == e`, or `e == x`, where `e` is not
       wide: *T* = {x};
     - `x !== e`, `e !== x`, `x != e`, or `e != x`, where `e` is not
       wide: *F* = {x};
     - `!g`: *T* and *F* of `g` swap;
     - `a && b`, `a || b`: the union of the sets of `a` and `b`;
     - `(g)`: the sets of `g`.
     The references of *T*(g) are tested in the `then` branch of `if`,
     the body of `while` and `for`, the right operand of `g && b`, and
     the `?` branch of `g ? a : b`. The references of *F*(g) are tested
     in the `else` branch, the right operand of `g || b`, and the `:`
     branch. After an `if` whose `then` branch holds a `return`,
     `break`, or `continue`, the references of *F*(g) are tested for
     the rest of the block. If the `else` branch holds one, the same
     applies to *T*(g). After a `while` or a `for`, the references of
     *F*(g) are tested for the rest of the block.
  A test from an outer region stays in force in each inner region. An
  assignment does not end a test.

Accept: `a10`, `a14`, `a28`, `a29`. Reject: `r16`, `r17`, `r70`, `r71`, `r72`, `r73`,
`r74`.

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
2. The lowering copies each such binding into a local of the body at
   the start of each iteration. It copies the local back before the
   update. The copy-back runs on every path to the update, `continue`
   included.
3. A closure anywhere in a `for` head (initializer, condition, update)
   is rejected. In JavaScript a closure in the head binds to one
   specific iteration's binding, which the body-local copy of rule 2
   does not give.
4. Each iteration of `for…of` gives its `const` binding a new binding.

Accept: `a07`, `a09`, `a17`. Reject: `r42`, `r49`.

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
   declaration that reaches B. The relation is transitive over chains
   of any length. If F reaches B, every reference to F must come after
   the end of the declaration of B in source order. A reference is a
   call or a use as a value, at any nesting depth.

Reject: `r20`, `r35`, `r36`, `r40`, `r41`, `r50`, `r51`, `r56`.

## C11. The accepted surface is closed

`surface.md` lists every accepted form. Every other syntax form, type
form, global, namespace, and member is rejected. A rule in this file
explains a listed form or a rejection near the list. It does not make
a form acceptable.

Accept: `a27`. Reject: `r46`, `r47`, `r55`, `r59`, `r60`, `r61`, `r62`, `r65`, `r66`,
`r75`.

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

Instances of the rule, each pinned in `a18`:

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

Accept: `a18`.

## C13. Construction

1. A class field has a type annotation. It has an initializer, or the
   constructor assigns it.
2. A field initializer does not use `this`.
3. In a constructor, until every field without an initializer has an
   assignment on every path, `this` appears only as the target
   `this.f = e`, and `e` does not use `this`. JavaScript reads
   `undefined` from an unassigned field, outside its type.

Accept: `a16`. Reject: `r52`.

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
   function. The register budget of an expression is the register
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

1. A null test (`x != null`, `x == null`, with `!`, `&&`, `||`, `?:`,
   `if`, `while`, and an early `return`, `break`, or `continue`) narrows
   a reference `x` on the paths where the test holds.
2. `x` is a `const` binding, or a `let` binding or a parameter that no
   function or arrow assigns. A module-level `let` is never narrowed.
3. The narrowing ends at the next assignment to `x` on that path.
4. The narrowing of a `let` binding or a parameter does not continue
   into a nested function or arrow body. The body can run after a later
   assignment. The narrowing of a `const` binding continues.
5. A field, an element, and every other reference are never narrowed.
   Copy the value into a `const` to narrow it.

Accept: `a14`. Reject: `r67`, `r68`, `r69`.

## Open items

- P2 decides structural or nominal typing between object types and
  classes (C1.5).
- P2 decides array methods beyond `push` and string byte operations
  beyond `Utf8.byteLength`.
