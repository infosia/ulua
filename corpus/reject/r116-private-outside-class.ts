// corpus: reject/r116-private-outside-class
// purpose: Rejects a read of a private field outside the class body.
// rule: C11
// phase: P2.6
// tsc: errors TS2341
// expected-error: C11: a private member is read outside its class
class Counter {
  private value: number = 1;

  get(): number {
    return this.value;
  }
}

export function main(): void {
  const c = new Counter();
  print(`${c.value}`);
}
