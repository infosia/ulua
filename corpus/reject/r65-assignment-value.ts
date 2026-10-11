// corpus: reject/r65-assignment-value
// purpose: Rejects the value of an assignment used as an operand.
// rule: C11
// tsc: accepts
// expected-error: C11: an assignment is a statement only
export function main(): void {
  let n = 0;
  print(`${(n = 5) + 1}`);
}
