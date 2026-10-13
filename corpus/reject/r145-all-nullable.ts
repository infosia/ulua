// corpus: reject/r145-all-nullable
// purpose: Rejects Promise.all over promises of a nullable type.
// rule: C19
// phase: P4.2
// tsc: accepts
// expected-error: C19: Promise.all needs a promise type that is not nullable or void
async function f(): Promise<number | null> {
  return null;
}
export async function main(): Promise<void> {
  const xs = await Promise.all([f(), f()]);
  print(`${xs.length}`);
}
