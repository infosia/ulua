// corpus: reject/r22-bitwise
// purpose: Rejects a bitwise operator.
// rule: C4
// phase: P1
// tsc: accepts
// expected-error: C4: bitwise operators have no lowering
export function main(): void {
  const a = -1;
  print(`${a >>> 0}`);
}
