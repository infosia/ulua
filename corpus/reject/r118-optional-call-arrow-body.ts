// corpus: reject/r118-optional-call-arrow-body
// purpose: Rejects an optional method call as an arrow expression body; tsc types it R | undefined.
// rule: C2
// phase: P2.6
// tsc: errors TS2322
// expected-error: C2: an optional call is only an expression statement
class D {
  n: number = 0;

  m(): number {
    this.n++;
    return this.n;
  }
}

type R = { d: D | null };

export function main(): void {
  const r: R = { d: new D() };
  const f = (): void => r.d?.m();
  f();
  print("x");
}
