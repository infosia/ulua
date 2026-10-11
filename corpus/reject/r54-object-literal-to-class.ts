// corpus: reject/r54-object-literal-to-class
// purpose: Rejects an object literal in a position of a class type.
// rule: C1
// phase: P2.5
// tsc: accepts
// expected-error: C1: an object type and a class do not relate
class Adder {
  add(n: number): number {
    return n + 1;
  }
}

export function main(): void {
  const a: Adder = { add: (n: number): number => n + 2 };
  print(`${a.add(1)}`);
}
