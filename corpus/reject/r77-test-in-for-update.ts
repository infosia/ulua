// corpus: reject/r77-test-in-for-update
// purpose: Rejects a comparison in a for update with a reference that the loop test narrowed.
// rule: C7
// phase: P1
// tsc: errors TS2367
// expected-error: C7: an equality needs a wide operand
function count(c: boolean): number {
  let n = 0;
  for (let i = 0; i === 0; i = i === 1 ? 5 : 6) {
    n = n + 1;
  }
  return n;
}

export function main(): void {
  print(`${count(true)}`);
}
