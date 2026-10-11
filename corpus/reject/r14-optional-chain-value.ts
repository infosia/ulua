// corpus: reject/r14-optional-chain-value
// purpose: Rejects an optional chain whose undefined result is kept as a value.
// rule: C2
// phase: P2.4
// tsc: accepts
// expected-error: C2: an optional chain must be the left operand of ?? or a call statement
type Box = { size: number };

function report(box: Box | null): void {
  const size = box?.size;
  print(`${size == null}`);
}

export function main(): void {
  report(null);
}
