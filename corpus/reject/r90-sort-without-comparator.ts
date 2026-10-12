// corpus: reject/r90-sort-without-comparator
// purpose: Rejects sort without a comparator, which compares elements as strings.
// rule: C6
// phase: P2.3
// tsc: accepts
// expected-error: C6: sort needs a comparator
export function main(): void {
  const xs: number[] = [10, 1, 2];
  xs.sort();
  print(`${xs[0]}`);
}
