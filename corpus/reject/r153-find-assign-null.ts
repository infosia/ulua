// corpus: reject/r153-find-assign-null
// purpose: Rejects null assigned to a binding that holds a find result.
// rule: C2
// phase: P7.2
// tsc: errors TS2322
// expected-error: C2: null is not assignable to a find result
export function main(): void {
  const xs: number[] = [1, 2, 3];
  let r = xs.find((v: number): boolean => v > 5);
  r = null;
  print(`${r ?? 0}`);
}
