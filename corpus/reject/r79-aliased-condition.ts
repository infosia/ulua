// corpus: reject/r79-aliased-condition
// purpose: Rejects a comparison with a reference that an aliased condition narrowed.
// rule: C7
// phase: P1
// tsc: errors TS2367
// expected-error: C7: an equality needs a wide operand
function check(x: number): boolean {
  const isOne = x === 1;
  if (isOne) {
    return x === 2;
  }
  return false;
}

export function main(): void {
  print(`${check(1)}`);
}
