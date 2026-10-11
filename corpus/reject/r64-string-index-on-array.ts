// corpus: reject/r64-string-index-on-array
// purpose: Rejects an array index that is not a number.
// rule: C6
// phase: P2.2
// tsc: accepts
// expected-error: C6: an array index has type number
export function main(): void {
  const xs: number[] = [1, 2, 3];
  print(`${xs["length"]}`);
}
