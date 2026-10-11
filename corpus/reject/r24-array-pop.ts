// corpus: reject/r24-array-pop
// purpose: Rejects an array method whose result type includes undefined.
// rule: C6
// tsc: accepts
// expected-error: C6: pop gives undefined on an empty array
export function main(): void {
  const xs: number[] = [1];
  const last = xs.pop();
  print(`${last}`);
}
