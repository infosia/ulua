// corpus: reject/r122-error-stack
// purpose: Rejects the stack member of an Error value.
// rule: C17
// phase: P2.7
// tsc: accepts
// expected-error: C17: an Error value has only name and message
export function main(): void {
  const e = new Error("m");
  print(`${e.stack}`);
}
