// corpus: reject/r17-loose-equality
// purpose: Rejects loose equality with no wide operand, as for strict equality.
// rule: C7
// phase: P1
// tsc: accepts
// expected-error: C7: an equality needs a wide operand
export function main(): void {
  const a = 1;
  const b = 1;
  print(`${a == b}`);
}
