// corpus: reject/r89-array-literal-equality
// purpose: Rejects an equality with an array literal operand, which compares a new array by identity.
// rule: C7
// phase: P2.2
// tsc: errors TS2839
// expected-error: C7: an array literal is not an equality operand
export function main(): void {
  const a = [5, 2];
  print(`${a === [5, 2]}`);
}
