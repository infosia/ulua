// corpus: reject/r72-tested-reference-equality
// purpose: Rejects an equality between a tested reference and a literal; tsc narrows the reference.
// rule: C7
// phase: P1
// tsc: errors TS2367
// expected-error: C7: an equality needs a wide operand
function check(n: number): boolean {
  return n === 1 && n === 2;
}

export function main(): void {
  print(`${check(1)}`);
}
