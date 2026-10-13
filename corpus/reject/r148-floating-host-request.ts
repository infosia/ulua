// corpus: reject/r148-floating-host-request
// purpose: Rejects an expression statement whose value is the promise of an async host function.
// rule: C19
// phase: P4.3
// grants: hostLater
// tsc: accepts
// expected-error: C19: a promise value is stored, passed, returned, or awaited
export function main(): void {
  hostLater(1);
}
