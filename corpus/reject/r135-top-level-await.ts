// corpus: reject/r135-top-level-await
// purpose: Rejects await at module level.
// rule: C19
// phase: P4.1
// tsc: accepts
// expected-error: C19: await is only accepted in an async body
async function f(): Promise<number> {
  return 1;
}
const x = await f();
export function main(): void {
  print(`${x}`);
}
