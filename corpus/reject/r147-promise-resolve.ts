// corpus: reject/r147-promise-resolve
// purpose: Rejects a static member of Promise other than all.
// rule: C19
// phase: P4.2
// tsc: accepts
// expected-error: C19: the Promise identifier is not a value
export async function main(): Promise<void> {
  const p = Promise.resolve(1);
  print(`${await p}`);
}
