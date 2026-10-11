// corpus: trap/t02-index-write-gap
// purpose: Traps on an array write that would leave a hole.
// rule: C6
// phase: P2.2
// tsc: accepts
// js-comparable: yes
// trap: index-out-of-range
export function main(): void {
  const xs: number[] = [10, 20];
  xs[2] = 30;
  print(`${xs.length}`);
  xs[4] = 50;
  print(`${xs.length}`);
}
