// corpus: reject/r115-coalesce-test-edge
// purpose: Rejects a boolean ?? whose right operand has no false edge; tsc ends the edge that JavaScript reaches.
// rule: C16
// phase: P2.5
// tsc: errors TS18047
// expected-error: C16: the right operand of a ?? test needs both edges
type B = { v: number };

function f(p: B | null, c: boolean, bn: boolean | null): void {
  const x: B | null = p;
  if (x == null) {
    return;
  }
  if (bn ?? (true || c)) {
    return;
  }
  print(`${x.v}`);
}

export function main(): void {
  f({ v: 2 }, false, false);
  f({ v: 3 }, false, null);
}
