// corpus: reject/r72-literal-set-equality
// purpose: Rejects an equality whose operand literal types do not overlap after `||`.
// rule: C7
// phase: P1
// tsc: errors TS2367
// expected-error: C7: the literal types of the operands do not overlap
function check(p: boolean): boolean {
  return (p || true) === false;
}

export function main(): void {
  print(`${check(false)}`);
}
