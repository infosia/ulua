// corpus: reject/r53-class-to-object-type
// purpose: Rejects a class instance in a position of an object type.
// rule: C1
// phase: P2.5
// tsc: accepts
// expected-error: C1: an object type and a class do not relate
type HasAdd = { add: (n: number) => number };

class Adder {
  add(n: number): number {
    return n + 1;
  }
}

export function main(): void {
  const h: HasAdd = new Adder();
  print(`${h.add(1)}`);
}
