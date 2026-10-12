// corpus: reject/r93-index-of-function
// purpose: Rejects indexOf on an array of functions, which compares closures by identity.
// rule: C6
// phase: P2.3
// tsc: accepts
// expected-error: C6: indexOf does not take a function element type
export function main(): void {
  const f = (): number => 1;
  const fs: (() => number)[] = [f];
  print(`${fs.indexOf(f)}`);
}
