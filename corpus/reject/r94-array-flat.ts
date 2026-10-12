// corpus: reject/r94-array-flat
// purpose: Rejects flat, whose result type depends on a runtime depth.
// rule: C6
// phase: P2.3
// tsc: accepts
// expected-error: C6: flat is not an accepted member
export function main(): void {
  const xs: number[][] = [[1], [2]];
  print(`${xs.flat().length}`);
}
