// corpus: reject/r23-array-length-write
// purpose: Rejects a write to the length of an array.
// rule: C6
// tsc: accepts
// expected-error: C6: the length of an array is read-only
export function main(): void {
  const xs: number[] = [1, 2, 3];
  xs.length = 1;
  print(`${xs.length}`);
}
