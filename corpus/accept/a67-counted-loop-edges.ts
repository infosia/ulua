// corpus: accept/a67-counted-loop-edges
// purpose: Counted for loops with non-integer, NaN, negative, and equal bounds, both directions and steps.
// rule: C9
// phase: P4.3
// tsc: accepts
// js-comparable: yes
function count(b: number): number {
  let n = 0;
  for (let i = 0; i < b; i++) {
    n = n + i;
  }
  return n;
}

function countDown(b: number): number {
  let n = 0;
  for (let i = 10; i >= b; i -= 3) {
    n = n * 2 + i;
  }
  return n;
}

export function main(): void {
  print(`${count(4)} ${count(3.5)} ${count(0)} ${count(-2)} ${count(0.5)} ${count(NaN)}`);
  print(`${countDown(1)} ${countDown(1.5)} ${countDown(10)} ${countDown(11)} ${countDown(-0.5)} ${countDown(NaN)}`);
  let total = 0;
  for (let i = 2; i <= 9; i += 2) {
    total = total + i;
  }
  for (let j = 5; j > 0; j--) {
    total = total * 3 - j;
  }
  print(`${total}`);
}
