// corpus: reject/r170-await-in-map-traversal
// purpose: Rejects await in the body of a for-of over a Map or Set; a suspended traversal outlives its run.
// rule: C6
// phase: P7.4
// tsc: accepts
// expected-error: C6: await in a Map or Set traversal is rejected
async function value(n: number): Promise<number> {
  return n;
}
export async function main(): Promise<void> {
  const m = new Map<number, number>();
  m.set(1, 1).set(2, 2);
  let total = 0;
  for (const k of m.keys()) {
    total += await value(k);
  }
  print(`${total}`);
}
