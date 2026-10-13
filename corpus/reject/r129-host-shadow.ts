// corpus: reject/r129-host-shadow
// purpose: Rejects a binding that shadows a host function name.
// rule: C18
// phase: P3
// grants: hostAdd
// tsc: accepts
// expected-error: C18: a binding has the name of a host function
declare function hostAdd(a: number, b: number): number;
function twice(hostAdd: number): number {
  return hostAdd * 2;
}
export function main(): void {
  print(`${twice(hostAdd(1, 2))}`);
}
