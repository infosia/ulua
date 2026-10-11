// corpus: reject/r20-temporal-dead-zone
// purpose: Rejects a read of a const binding that comes before its declaration.
// rule: C10
// tsc: accepts
// expected-error: C10: a binding is used before its declaration
export function main(): void {
  const g = (): number => y;
  print(`${g()}`);
  const y = 1;
}
