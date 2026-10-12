// corpus: reject/r91-reduce-without-initial
// purpose: Rejects reduce without an initial value.
// rule: C6
// phase: P2.3
// tsc: accepts
// expected-error: C6: reduce needs an initial value
export function main(): void {
  const xs: number[] = [1, 2];
  print(`${xs.reduce((a: number, v: number): number => a + v)}`);
}
