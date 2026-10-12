// corpus: trap/t05-array-modified
// purpose: Traps when a callback changes the length of the array that the method iterates.
// rule: C6
// phase: P2.3
// tsc: accepts
// js-comparable: yes
// trap: array-modified
export function main(): void {
  const xs: number[] = [1, 2];
  xs.forEach((v: number): void => {
    print(`${v}`);
    xs.push(v);
  });
  print(`${xs.length}`);
}
