// corpus: trap/t17-proven-index-at-length
// purpose: An index with an integer fact keeps the bound test: a read at the length traps.
// rule: C6
// phase: P6
// tsc: accepts
// js-comparable: yes
// trap: index-out-of-range
export function main(): void {
  const xs: number[] = [10, 20, 30];
  let total = 0;
  for (let i = 0; i <= xs.length; i++) {
    const v = xs[i];
    total += v;
    print(`${total}`);
  }
}
