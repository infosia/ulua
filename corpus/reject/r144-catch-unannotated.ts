// corpus: reject/r144-catch-unannotated
// purpose: Rejects a catch callback parameter without the unknown annotation.
// rule: C19
// phase: P4.2
// tsc: accepts
// expected-error: C19: a catch callback parameter is annotated unknown
async function f(): Promise<number> {
  throw new Error("x");
}
export async function main(): Promise<void> {
  const p = f().catch((e): number => 0);
  print(`${await p}`);
}
