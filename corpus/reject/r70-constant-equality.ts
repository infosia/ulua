// corpus: reject/r70-constant-equality
// purpose: Rejects an equality between two constants of different values.
// rule: C7
// phase: P1
// tsc: errors TS2367
// expected-error: C7: the operands are constants of different values
export function main(): void {
  const a = 1;
  const b = 2;
  print(`${a === b}`);
}
