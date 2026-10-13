// corpus: reject/r134-floating-promise
// purpose: Rejects an expression statement whose value is a promise.
// rule: C19
// phase: P4.1
// tsc: accepts
// expected-error: C19: a promise value is stored, passed, returned, or awaited
async function f(): Promise<number> {
  return 1;
}
export function main(): void {
  f();
}
