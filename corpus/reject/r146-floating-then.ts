// corpus: reject/r146-floating-then
// purpose: Rejects an expression statement whose value is the promise that then gives.
// rule: C19
// phase: P4.2
// tsc: accepts
// expected-error: C19: a promise value is stored, passed, returned, or awaited
async function f(): Promise<number> {
  return 1;
}
export function main(): void {
  f().then((v: number): void => {
    print(`${v}`);
  });
}
