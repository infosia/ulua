// corpus: reject/r131-host-optional-call
// purpose: Rejects an optional call of a host function.
// rule: C18
// phase: P3
// grants: hostAdd
// tsc: accepts
// expected-error: C18: a host function is only called
export function main(): void {
  print(`${hostAdd?.(1, 2)}`);
}
