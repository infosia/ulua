// corpus: reject/r01-any
// purpose: Rejects any in an annotation.
// rule: C8
// phase: P1
// tsc: accepts
// expected-error: C8: any has no checked representation
const value: any = 1;

export function main(): void {
  print(`${value}`);
}
