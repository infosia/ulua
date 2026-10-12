// corpus: accept/a35-type-based-equality
// purpose: Compares operands of one primitive type, including constants, narrowed references, aliases, and NaN.
// rule: C7
// phase: P2.3
// tsc: exception TS2367, TS2845
// js-comparable: yes
let calls = 0;

function bump(n: number): number {
  calls += 1;
  return n;
}

function check(a: boolean, b: boolean, x: number): string {
  const one = 1;
  const two = 2;
  const isOne = x === 1;
  let out = `${a === b} ${a !== b} ${a == b} ${one === two} ${"x" !== "y"}`;
  if (x === 1) {
    out = `${out} ${x === 2}`;
  }
  if (isOne) {
    out = `${out} ${x === 3}`;
  }
  out = `${out} ${NaN === NaN} ${x === NaN} ${true === false}`;
  out = `${out} ${bump(x) === bump(1)} ${calls}`;
  return out;
}

export function main(): void {
  print(check(true, false, 1));
}
