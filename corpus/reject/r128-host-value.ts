// corpus: reject/r128-host-value
// purpose: Rejects a host function used as a value.
// rule: C18
// phase: P3
// grants: hostAdd
// tsc: accepts
// expected-error: C18: a host function is only called
export function main(): void {
  const f = hostAdd;
  print(`${f(1, 2)}`);
}
