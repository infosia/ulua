// corpus: reject/r126-host-no-grant
// purpose: Rejects a host function declaration that the compilation does not grant.
// rule: C18
// phase: P3
// tsc: accepts
// expected-error: C18: a declared host function has no grant
declare function hostAdd(a: number, b: number): number;
export function main(): void {
  print(`${hostAdd(1, 2)}`);
}
