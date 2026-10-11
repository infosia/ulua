// corpus: reject/r83-alias-test-bound
// purpose: Rejects an aliased condition that tests more references than the alias bound.
// rule: C7
// phase: P1
// tsc: accepts
// expected-error: C7: an aliased condition tests at most 8 references
function check(a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number): boolean {
  const all = a === 1 && b === 1 && c === 1 && d === 1 && e === 1 && f === 1 && g === 1 && h === 1 && i === 1;
  return all;
}

export function main(): void {
  print(`${check(1, 1, 1, 1, 1, 1, 1, 1, 1)}`);
}
