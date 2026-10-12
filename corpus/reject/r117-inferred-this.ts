// corpus: reject/r117-inferred-this
// purpose: Rejects this at an inference position; tsc gives the binding the polymorphic this type.
// rule: C11
// phase: P2.6
// tsc: errors TS2322
// expected-error: C11: an inferred type cannot come from a narrowable reference
class A {
  n: number = 0;

  swap(): number {
    let t = this;
    t = new A();
    t.n = 5;
    return this.n;
  }
}

export function main(): void {
  print(`${new A().swap()}`);
}
