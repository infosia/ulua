// corpus: reject/r126-host-no-grant
// purpose: Rejects a call of a name that the compilation does not grant.
// rule: C18
// phase: P3
// tsc: errors TS2304
// expected-error: C18: the name has no binding and no grant
export function main(): void {
  print(`${hostAdd(1, 2)}`);
}
