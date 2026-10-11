// corpus: reject/r70-constant-equality
// purpose: Rejects an equality with no wide operand: two constants.
// rule: C7
// phase: P1
// tsc: errors TS2367
// expected-error: C7: an equality needs a wide operand
export function main(): void {
  const a = 1;
  const b = 2;
  print(`${a === b}`);
}
