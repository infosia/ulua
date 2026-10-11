// corpus: reject/r15-exponent
// purpose: Rejects the exponent operator.
// rule: C4
// phase: P1
// tsc: accepts
// expected-error: C4: ** has no lowering that agrees with JavaScript
export function main(): void {
  const zero = 0;
  print(`${1 ** (zero / zero)}`);
}
