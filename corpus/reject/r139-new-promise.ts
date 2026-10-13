// corpus: reject/r139-new-promise
// purpose: Rejects the Promise constructor.
// rule: C19
// phase: P4.1
// tsc: accepts
// expected-error: C19: the Promise identifier is not a value
export async function main(): Promise<void> {
  const p = new Promise<number>((resolve) => resolve(1));
  print(`${await p}`);
}
