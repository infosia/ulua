// corpus: reject/r21-string-plus-number
// purpose: Rejects + between a string and a number.
// rule: C4
// phase: P2.4
// tsc: accepts
// expected-error: C4: + between a string and a non-string operand
export function main(): void {
  const small = 1e-7;
  print("a" + small);
}
