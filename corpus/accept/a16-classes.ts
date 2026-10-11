// corpus: accept/a16-classes
// purpose: Constructs a class instance and calls methods that change its fields.
// rule: C1, C13
// phase: P2
// tsc: accepts
class Counter {
  private value: number;

  constructor(start: number) {
    this.value = start;
  }

  add(n: number): number {
    this.value += n;
    return this.value;
  }
}

export function main(): void {
  const c = new Counter(40);
  c.add(1);
  print(`${c.add(1)}`);
}
