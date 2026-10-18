// corpus: trap/t21-integral-index-fraction
// purpose: A write of a non-integer removes the integral mark of a signed index: the floor test stays and a fraction traps.
// rule: C6
// phase: P6
// tsc: accepts
// js-comparable: yes
// trap: index-out-of-range
export function main(): void {
  const xs: number[] = [10, 20, 30];
  let h = 3;
  h = h - 4;
  print(`${xs[h + 1]}`);
  h = h / 2 + 1;
  print(`${h}`);
  print(`${xs[h]}`);
}
