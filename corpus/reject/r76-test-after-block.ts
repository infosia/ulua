// corpus: reject/r76-test-after-block
// purpose: Rejects a comparison with a reference that a test inside a closed block narrowed.
// rule: C7
// phase: P1
// tsc: errors TS2367
// expected-error: C7: an equality needs a wide operand
function check(x: number): boolean {
  {
    if (x !== 1) {
      return false;
    }
  }
  return x === 2;
}

export function main(): void {
  print(`${check(1)}`);
}
