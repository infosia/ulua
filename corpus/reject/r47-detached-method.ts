// corpus: reject/r47-detached-method
// purpose: Rejects a method read as a value.
// rule: C11
// phase: P2
// tsc: accepts
// expected-error: C11: a method is never read as a value
class Counter {
  private value: number = 1;

  get(): number {
    return this.value;
  }
}

export function main(): void {
  const c = new Counter();
  const read = c.get;
  print(`${read()}`);
}
