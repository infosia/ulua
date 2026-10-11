// corpus: trap/t03-index-non-integer
// purpose: Traps on a non-integer index whose lowered key rounds to an integer.
// rule: C6
// phase: P2
// tsc: accepts
// trap: index-out-of-range
export function main(): void {
  const xs: number[] = [10, 20];
  const i = 0.9999999999999999;
  print(`${xs[0]}`);
  print(`${xs[i] + 1}`);
}
