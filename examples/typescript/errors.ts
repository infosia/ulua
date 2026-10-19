// Errors. `throw`, `try`, `catch`, and `finally` work as in JavaScript,
// with two limits: you throw only `Error` objects, and some failures stop
// the program before any `catch` can see them.

function parseAge(text: string): number {
  const n = parseInt(text, 10);
  if (n != n) {
    // Throw an `Error` or one of its built-in kinds: `TypeError`,
    // `RangeError`, `SyntaxError`, and the others. Ulua does not accept
    // `throw "text"`, so a `catch` always receives an error object.
    throw new TypeError(`not a number: ${text}`);
  }
  if (n < 0) {
    throw new RangeError(`negative age: ${n}`);
  }
  return n;
}

function describe(text: string): string {
  try {
    return `age ${parseAge(text)}`;
  } catch (e) {
    // The type of `e` is `unknown`. Ask what it is with `instanceof`
    // before you read `message`.
    if (e instanceof RangeError) {
      return `out of range: ${e.message}`;
    }
    if (e instanceof Error) {
      return `${e.name}: ${e.message}`;
    }
    return "unknown error";
  } finally {
    // `finally` runs after `try` or `catch`, as in JavaScript.
    print(`checked "${text}"`);
  }
}

export function main(): void {
  print(describe("42"));
  print(describe("-1"));
  print(describe("old"));

  // A bug such as a read outside an array is not an exception in Ulua.
  // It stops the program at once, and no `catch` or `finally` runs.
  // The host program decides what happens next.

  // An exception that no `catch` handles also stops the program. The
  // host receives the error name and message.
  parseAge("-5");
  print("this line never runs");
}
