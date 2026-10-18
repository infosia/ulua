// corpus: trap/t20-integral-index-negative
// purpose: An index with an integral mark keeps the test i >= 0: a down counter that passes 0 traps.
// rule: C6
// phase: P6
// tsc: accepts
// js-comparable: yes
// trap: index-out-of-range
export function main(): void {
  const xs: number[] = [10, 20, 30];
  let total = 0;
  for (let j = xs.length - 1; j >= -1; j--) {
    total += xs[j];
    print(`${total}`);
  }
}
