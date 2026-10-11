// corpus: reject/r35-hoisted-call-before-binding
// purpose: Rejects a call of a hoisted function before a binding that the function reads.
// rule: C10
// phase: P2
// tsc: accepts
// expected-error: C10: the callee reads a binding that is not declared yet
export function main(): void {
  show();
  const label = "ready";
  function show(): void {
    print(label);
  }
}
