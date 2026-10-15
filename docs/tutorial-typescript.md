# Ulua for TypeScript developers

Ulua writes the logic that runs inside a native application. The
application owns the process and the main loop. It calls the functions
that your script exports, and it decides which host functions your
script can call.

The syntax is a subset of TypeScript. Every accepted program also
type-checks under stock `tsc`, so tsserver gives you completion,
rename, and go-to-definition with no plugin. Below the syntax, the
values are plain Luau values: a `number` is a Luau number, a `string`
is a UTF-8 Luau string, an array is a Luau table. Where JavaScript and
Luau agree, Ulua prints what `node` prints. Where they cannot agree,
the compiler rejects the form, or the language names the difference.

This page is the list of what changes. Every program and every output
on this page was run against the repository as committed, and every
program type-checks under stock `tsc --strict`.

## The short version

| In TypeScript | In Ulua |
|---|---|
| `number`, `bigint` | `number` only (a binary64 double) |
| `undefined`, `T \| U` | `T \| null` only; `undefined` does not exist |
| Structural types, `interface` | `type` aliases of object literal types; same fields, same type |
| `class` with `extends` | Nominal classes, no inheritance, no getters or setters |
| `any`, generics | Neither; every type is concrete |
| `==` coerces, so style guides prefer `===` | `==` is `===`: one operand type, no coercion; this guide writes `==` |
| `throw` any value | The seven `Error`-family classes only |
| A fault such as `xs[10]` | A trap: the run stops, and no `catch` or `finally` sees it |
| Event loop, `Promise` | `Promise<T>` in the job order of `node`; the host drives each run |
| `string` of UTF-16 units | `string` of UTF-8 bytes; lengths and offsets count bytes |
| `globalThis`, `Math`, `JSON` | Not in the library; the host grants what a script needs |
| A program with a top level | Exported functions that the host calls |
| npm, `import` | One module per compilation |

## Setup

The examples use the runner in [`examples/run/`](../examples/run/).
It compiles a script with no host functions, calls its export `main`,
and prints each `print` line.

```ts
export function main(): void {
  print("hello from Ulua");
}
```

Build the library and the runner from the repository root (macOS; the
Linux link line is in the comment at the top of
`examples/run/host.c`):

```sh
export LUAU_SOURCE_DIR=/path/to/luau    # checkout at luau-revision.txt
cargo build --release -p ulua
mkdir -p target/examples
cc -std=c11 -Wall -Wextra -Wpedantic -Werror examples/run/host.c -I include \
    target/release/libulua.a -o target/examples/run \
    -lc++ -liconv -framework Security -framework CoreFoundation
alias run=target/examples/run
```

```sh
$ run hello.ts
hello from Ulua
```

The runner exits with 0 when the run ends, 1 when the compiler
rejects the script, and 2 when the run stops.

`print` writes one line to an output function that the host owns.
There is no `console`.

## Numbers are doubles

`number` is the only numeric type. Arithmetic, `%`, the infinities,
`NaN`, and the text of a number in a template agree with `node`:

```ts
export function main(): void {
  const a: number = 7;
  print(`${a / 2} ${a % 3} ${-a % 3} ${0.1 + 0.2}`);
  print(`${1 / 0} ${-1 / 0} ${0 / 0} ${String(1e21)}`);
}
```

```text
3.5 1 -1 0.30000000000000004
Infinity -Infinity NaN 1e+21
```

`**`, `Math`, and `Number(...)` are not in the subset. `**` gives
`C4` ("Numeric operator has no lowering"): Luau's `^` follows C `pow`,
which differs from JavaScript on some inputs. There is no `bigint`.

## `T | null` is the only union

There is no `undefined`. An optional parameter, a `T | undefined`
type, and every library call whose miss is `undefined` in JavaScript
are rejected. A value that can be absent has the type `T | null`, and
you test it before you use it:

```ts
type User = { name: string; email: string | null };

function contact(u: User): string {
  const email = u.email;
  if (email != null) {
    return email;
  }
  return `${u.name} has no email`;
}

export function main(): void {
  print(contact({ name: "ada", email: "ada@example.com" }));
  print(contact({ name: "bob", email: null }));
  const i = ["x", "y"].indexOf("z");
  print(`${i}`);
}
```

```text
ada@example.com
bob has no email
-1
```

The checker narrows only where nothing else can write the value
between the test and the read. A `const` narrows. A `let` or a
parameter narrows if no function assigns it. A field never narrows:
copy it into a `const` first, as `contact` does. An `await` ends the
narrowing of a binding that another function can assign while the
body waits.

`find` is rejected (`C6`, "Array method is rejected"): it gives
`undefined` for a miss. Use `findIndex`, or `indexOf`, which give
`-1`.

## Records and classes

A record is a `type` alias of an object literal type. Two record types
relate when their fields are the same, with the same types. A class
is nominal: a class value is never a record, and two classes with the
same fields are two types.

```ts
class Account {
  private balance: number = 0;
  constructor(owner: string) {
    print(`open ${owner}`);
  }
  deposit(n: number): number {
    this.balance += n;
    return this.balance;
  }
}

export function main(): void {
  const a = new Account("ada");
  print(`${a.deposit(10)} ${a.deposit(5)}`);
}
```

```text
open ada
10 15
```

There is no `extends`, no `implements`, no getter or setter, and no
`static` member. A field needs a type annotation. The constructor runs
its argument evaluation, then the field initializers, then its body,
as JavaScript does.

## Strings are UTF-8

A string is a Luau string of UTF-8 bytes. Every length, index, and
position counts bytes. On ASCII text the results equal JavaScript's.
On other text they differ, and this is a named divergence:

```ts
export function main(): void {
  const s = "héllo";
  print(`${s.length} ${s.indexOf("l")} ${s.slice(0, 3)}`);
  let n: number = 0;
  for (const c of s) {
    n++;
  }
  print(`${n} code points`);
  print(`${Utf8.byteLength("日本")}`);
}
```

```text
6 3 hé
5 code points
6
```

`node` prints `5 2 hél` on the first line. `for...of` steps one code
point at a time, as in JavaScript. A position that falls inside a
code point traps with `string-boundary`. `toUpperCase`,
`toLowerCase`, `localeCompare`, and `normalize` are rejected (`C5`,
"String member is rejected"): Luau has no Unicode case or
normalization data. `s[i]` and `<` between strings are rejected too:
the element of a JavaScript string is a UTF-16 unit, which has no
byte form.

## Arrays trap where JavaScript gives `undefined`

```ts
export function main(): void {
  const xs: number[] = [3, 1, 2];
  xs.sort((a: number, b: number): number => a - b);
  print(xs.join(","));
  print(`${xs.filter((x: number): boolean => x > 1).map((x: number): number => x * 10).join(",")}`);
  print(`${xs[5]}`);
}
```

```text
1,2,3
20,30
stopped trap:index-out-of-range: Index out of range
```

`node` prints `undefined` for `xs[5]`. In Ulua a read outside the
array stops the run: the value would be outside the static type.
Holes, spreads, and arrays of `T | null` elements are not in the
subset.

## Exceptions, and traps that no `catch` sees

`throw` takes a value of the `Error` family: `Error`, `TypeError`,
`RangeError`, `SyntaxError`, `ReferenceError`, `EvalError`, and
`URIError`. A `catch` binding has the type `unknown`; you test it
with `instanceof` before you read `name` or `message`.

```ts
function parse(text: string): number {
  if (text.length == 0) {
    throw new RangeError("empty input");
  }
  return text.length;
}

export function main(): void {
  try {
    print(`${parse("abc")}`);
    print(`${parse("")}`);
  } catch (e) {
    if (e instanceof RangeError) {
      print(`caught ${e.name}: ${e.message}`);
    }
  } finally {
    print("finally");
  }
}
```

```text
3
caught RangeError: empty input
finally
```

A trap, a limit, and a host error are *stops*, not exceptions. A stop
leaves the run before any `catch` and any `finally`:

```ts
export function main(): void {
  try {
    const xs: number[] = [1];
    print(`${xs[3]}`);
  } catch (e) {
    print("never printed");
  } finally {
    print("never printed either");
  }
}
```

```text
stopped trap:index-out-of-range: Index out of range
```

An exception that no `catch` handles stops the run with
`trap:uncaught-exception` and the message `name: message`.

## `async`, `await`, and promises

`async` functions, `await`, and `Promise<T>` values follow the job
order of `node`. The job queue drains after each run that the host
starts:

```ts
async function step(name: string, n: number): Promise<number> {
  print(`${name} start`);
  await later();
  print(`${name} resumed`);
  return n * 2;
}

async function later(): Promise<void> {}

export async function main(): Promise<void> {
  const a = step("a", 1);
  const b = step("b", 2);
  print("both started");
  const total = (await a) + (await b);
  print(`${total}`);
}
```

```text
a start
b start
both started
a resumed
b resumed
6
```

`then`, `catch`, `finally`, `Promise.all`, and `map` with an async
callback are in the subset:

```ts
async function square(n: number): Promise<number> {
  return n * n;
}

export async function main(): Promise<void> {
  const xs = await Promise.all([1, 2, 3].map((n: number): Promise<number> => square(n)));
  print(xs.join(","));
  const msg = await square(4)
    .then((v: number): string => `got ${v}`)
    .finally((): void => {
      print("cleanup");
    });
  print(msg);
}
```

```text
1,4,9
cleanup
got 16
```

The rules that differ from TypeScript:

- A promise is stored, passed, returned, or awaited. A statement that
  drops one is rejected (`C19`): JavaScript would lose its rejection.
- `await` takes a promise only, inside an `async` body only. There is
  no top-level `await`.
- `new Promise`, `Promise.resolve`, `Promise.race`, and the other
  statics are rejected; `Promise.all` is the one static.
- A `catch` callback annotates its parameter as `unknown`.
- A rejection that nothing handles at the end of a drain stops the
  run with `trap:uncaught-exception`, at the point where `node`
  reports `unhandledRejection`.

A host function can return a promise. The host completes the request
when its work is done, and the continuation runs in the next run that
the host starts. The [C tutorial](tutorial-c-cpp.md) shows the host
side.

## Host functions

A script calls the host only through the functions that the host
grants when it compiles the script. The host writes their
declarations with `ulua_declarations`, and you give that `.d.ts` file
to `tsc` and to your editor:

```ts
// game.d.ts, written by the host
declare function damage(p0: number): number;
declare function log(p0: string): void;
```

A host function is called, never used as a value: you cannot store it,
pass it, or compare it. A script declares nothing itself; every
`declare` form is rejected (`C18`).

## Exports are the host's entry points

A module exports zero or more functions. The host calls each one by
name with typed arguments: `number`, `boolean`, and `string`
parameters, and a `void`, `number`, `boolean`, `string`, or
`Promise<T>` result. `main` is only a convention of the runner and of
the corpus.

## What the compiler rejects

Each rejection names a rule of
[`specs/blocks/collisions.md`](../specs/blocks/collisions.md). The
table shows the code and the message of the real diagnostic:

| Form | Code | Message |
|---|---|---|
| `x: any` | `C8` | Type has no checked representation |
| `number \| undefined`, `number \| string` | `C2` | Only T \| null is a union |
| `f(x?: number)` | `C2` | Optional parameters are rejected |
| `2 ** 3` | `C4` | Numeric operator has no lowering |
| `[1].find(...)` | `C6` | Array method is rejected |
| `[...xs]` | `C6` | Array holes and spreads are rejected |
| `const [a, b] = xs` | `C11` | Binding patterns are rejected |
| `class B extends A` | `C11` | Invalid class declaration |
| `get v()` in a class | `C11` | Invalid class method |
| `enum E { ... }` | `C11` | Module statements must be declarations |
| `interface I { ... }` | `C8` | Interface declarations have no lowering |
| `function id<T>(x: T)` | `C8` | Generic declarations have no lowering |
| `typeof 1` | `C8` | Unary operator has no lowering |
| `Math.floor(1.5)` | `C11` | Math member has no lowering |
| `JSON.stringify(1)`, `Number("1")` | `C11` | Unknown value identifier |
| `throw "x"` | `C17` | Throw needs an Error value |
| `f();` where `f` returns a promise | `C19` | a promise value is stored, passed, returned, or awaited |
| a nullable `let` that another function assigns, read after a test | `C16` | Nullable receiver needs a stable null test |

## Diagnostics

A rejection gives a code, a message, and a source position. The
runner prints it as:

```ts
export function main(): void {
  const x: number = 2 ** 3;
  print(`${x}`);
}
```

```text
error C4 at 2:21: Numeric operator has no lowering
```

The line and the column are 1-based; the column counts bytes. A stop
prints `stopped <code>: <message>`, and the lines that the script
wrote before the stop stay in the output.

## Reading on

- [Ulua for C and C++ developers](tutorial-c-cpp.md): the host side.
- [`specs/blocks/surface.md`](../specs/blocks/surface.md): the closed
  list of accepted syntax.
- [`specs/blocks/collisions.md`](../specs/blocks/collisions.md): each
  rule, with the measured outputs of both sides for each divergence.
