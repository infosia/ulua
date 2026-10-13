// corpus: reject/r143-then-two-arguments
// purpose: Rejects then with a second argument.
// rule: C19
// phase: P4.2
// tsc: accepts
// expected-error: C19: then takes one argument
async function f(): Promise<number> {
  return 1;
}
export async function main(): Promise<void> {
  const p = f().then(
    (v: number): number => v + 1,
    (e: unknown): number => 0,
  );
  print(`${await p}`);
}
