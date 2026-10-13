// corpus: reject/r140-promise-variance
// purpose: Rejects a promise flow to a different promise type.
// rule: C19
// phase: P4.1
// tsc: accepts
// expected-error: C19: a promise relates only to an identical promise type
async function f(): Promise<number> {
  return 1;
}
export async function main(): Promise<void> {
  const p: Promise<number | null> = f();
  print(`${await p}`);
}
