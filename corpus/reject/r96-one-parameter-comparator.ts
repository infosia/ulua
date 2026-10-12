// corpus: reject/r96-one-parameter-comparator
// purpose: Rejects a sort comparator with one parameter, which is not consistent.
// rule: C6
// phase: P2.3
// tsc: accepts
// expected-error: C6: a sort comparator takes two parameters
export function main(): void {
  const xs: number[] = [3, 1, 2];
  xs.sort((x: number): number => x - 2);
  print(xs.join());
}
