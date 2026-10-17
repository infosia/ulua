// corpus: accept/a73-find-strict-null
// purpose: === null and !== null on a find miss: true and false here, false and true under node.
// rule: C2
// phase: P7.2
// tsc: accepts
// js-comparable: no C2
export function main(): void {
  const xs: number[] = [1, 2, 3];
  const miss = xs.find((v: number): boolean => v > 5);
  const hit = xs.find((v: number): boolean => v > 1);
  print(`${miss === null} ${miss !== null} ${hit === null} ${hit !== null}`);
}
