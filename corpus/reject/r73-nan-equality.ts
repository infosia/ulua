// corpus: reject/r73-nan-equality
// purpose: Rejects NaN as an equality operand.
// rule: C7
// phase: P1
// tsc: errors TS2845
// expected-error: C7: NaN is not an equality operand
function check(n: number): boolean {
  return n === NaN;
}

export function main(): void {
  print(`${check(1)}`);
}
