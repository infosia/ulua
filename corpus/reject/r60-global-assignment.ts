// corpus: reject/r60-global-assignment
// purpose: Rejects an assignment to a global name.
// rule: C11
// phase: P1
// tsc: accepts
// expected-error: C11: a global is not an assignment target
export function main(): void {
  Infinity = 5;
  print(`${Infinity}`);
}
