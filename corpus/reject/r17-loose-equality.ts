// corpus: reject/r17-loose-equality
// purpose: Rejects loose equality between two non-null operands.
// rule: C7
// phase: P1
// tsc: accepts
// expected-error: C7: == and != take a null operand only
export function main(): void {
  const a = 1;
  const b = 1;
  print(`${a == b}`);
}
