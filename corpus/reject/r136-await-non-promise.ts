// corpus: reject/r136-await-non-promise
// purpose: Rejects await of a value that is not a promise.
// rule: C19
// phase: P4.1
// tsc: accepts
// expected-error: C19: the operand of await has a Promise type
export async function main(): Promise<void> {
  const x = await 1;
  print(`${x}`);
}
