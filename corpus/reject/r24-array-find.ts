// corpus: reject/r24-array-find
// purpose: Rejects find, whose miss is undefined (D16).
// rule: C6
// phase: P2.2
// tsc: accepts
// expected-error: C6: find gives undefined on a miss
export function main(): void {
  const xs: number[] = [1];
  const hit = xs.find((v: number): boolean => v > 5);
  print(`${hit}`);
}
