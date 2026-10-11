// corpus: trap/t01-index-read-out-of-range
// purpose: Traps on an array read past the last element.
// rule: C6
// tsc: accepts
// trap: index-out-of-range
export function main(): void {
  const xs: number[] = [10, 20];
  print(`${xs[1]}`);
  const v = xs[2];
  print(`${v + 1}`);
}
