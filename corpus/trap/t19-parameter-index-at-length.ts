// corpus: trap/t19-parameter-index-at-length
// purpose: An index parameter with an integer fact keeps the bound test: a read at the length traps.
// rule: C6
// phase: P6
// tsc: accepts
// js-comparable: yes
// trap: index-out-of-range
function at(xs: number[], i: number): number {
  return xs[i];
}

export function main(): void {
  const xs: number[] = [10, 20, 30];
  let total = 0;
  for (let i = 0; i <= xs.length; i++) {
    total += at(xs, i);
    print(`${total}`);
  }
}
