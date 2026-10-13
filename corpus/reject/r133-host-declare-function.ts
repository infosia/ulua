// corpus: reject/r133-host-declare-function
// purpose: Rejects a script declaration of a host function; the grant list is its only record.
// rule: C18
// phase: P3
// grants: hostAdd
// tsc: accepts
// expected-error: C18: a script declares no host function
declare function hostAdd(a: number, b: number): number;
export function main(): void {
  print(`${hostAdd(1, 2)}`);
}
