// corpus: reject/r06-nullable-array-element
// purpose: Rejects an array whose element type includes null.
// rule: C6
// phase: P2
// tsc: accepts
// expected-error: C6: an array element type cannot include null
export function main(): void {
  const xs: (number | null)[] = [1, 2, null];
  print(`${xs.length}`);
}
