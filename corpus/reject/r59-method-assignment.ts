// corpus: reject/r59-method-assignment
// purpose: Rejects an assignment to a method of a class instance.
// rule: C11
// phase: P2
// tsc: accepts
// expected-error: C11: a method is not an assignment target
class C {
  get(): number {
    return 1;
  }
}

export function main(): void {
  const c = new C();
  c.get = (): number => 2;
  print(`${c.get()}`);
}
