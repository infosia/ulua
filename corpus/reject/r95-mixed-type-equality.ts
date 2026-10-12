// corpus: reject/r95-mixed-type-equality
// purpose: Rejects an equality between a number and a string; the type rule takes one type.
// rule: C7
// phase: P2.3
// tsc: errors TS2367
// expected-error: C7: equality operands have one type
function check(n: number, s: string): boolean {
  return n === s;
}

export function main(): void {
  print(`${check(1, "1")}`);
}
