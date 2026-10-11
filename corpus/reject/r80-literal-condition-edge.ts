// corpus: reject/r80-literal-condition-edge
// purpose: Rejects a comparison after an if whose literal true test has no false edge.
// rule: C7
// phase: P1
// tsc: errors TS2367
// expected-error: C7: an equality needs a wide operand
function check(x: number): boolean {
  if (x !== 1) {
    if (true) {
      return false;
    }
  }
  return x === 2;
}

export function main(): void {
  print(`${check(1)}`);
}
