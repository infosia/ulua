// corpus: trap/t04-pop-empty
// purpose: Traps on pop of an empty array, where JavaScript gives undefined.
// rule: C6
// phase: P2.3
// tsc: accepts
// js-comparable: yes
// trap: index-out-of-range
export function main(): void {
  const xs: number[] = [1];
  print(`${xs.pop()}`);
  const v = xs.pop();
  print(`${v + 1}`);
}
