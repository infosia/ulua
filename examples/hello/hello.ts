// A first Ulua script. The host (`host.c`) compiles this file, makes an
// instance, and calls the exports `main` and `greet` by name.
//
// The syntax is a TypeScript subset, so `tsc --strict` checks this file
// with the Ulua prelude (`prelude/ulua.d.ts`).

// A record: a `type` alias of an object literal type.
type Point = { x: number; y: number };

// A class with a field initializer and a method. Classes are nominal and
// have no `extends`.
class Counter {
  count: number = 0;
  add(n: number): number {
    this.count += n;
    return this.count;
  }
}

// A plain function. Every parameter and result has a type annotation.
function lengthSquared(p: Point): number {
  return p.x * p.x + p.y * p.y;
}

// The host calls `main` with no arguments. `print` writes one line
// through the output function that the host gives in `ulua_host`.
export function main(): void {
  print("hello from Ulua");
  // An array method with an arrow literal. The compiler writes it as a
  // plain loop.
  const words: string[] = ["alpha", "beta", "gamma"];
  print(words.map((w: string): string => `<${w}>`).join(" "));
  const c = new Counter();
  c.add(2);
  print(`count ${c.add(3)}`);
  print(`length squared ${lengthSquared({ x: 3, y: 4 })}`);
}

// An export with a string parameter and a string result. The host passes
// the argument as a `ulua_value` and reads the result from another one.
export function greet(name: string): string {
  return `hello, ${name}`;
}
