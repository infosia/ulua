// corpus: trap/t18-inline-callback-modified
// purpose: An arrow literal map callback whose body calls a function that grows the receiver keeps the length test and traps.
// rule: C6
// phase: P6
// tsc: accepts
// js-comparable: yes
// trap: array-modified
const xs: number[] = [1, 2];

function grow(v: number): number {
  xs.push(v);
  return v;
}

export function main(): void {
  const ys = xs.map((v: number): number => grow(v) * 10);
  print(`${ys.length}`);
}
