// corpus: reject/r137-promise-equality
// purpose: Rejects an equality of two promise values.
// rule: C19
// phase: P4.1
// tsc: accepts
// expected-error: C19: a promise value has no operator
async function f(): Promise<number> {
  return 1;
}
export async function main(): Promise<void> {
  const p = f();
  const q = p;
  print(`${p === q}`);
  print(`${await p}`);
}
