// corpus: reject/r41-hoisted-call-in-block
// purpose: Rejects a call in a nested block before a binding that the callee reads.
// rule: C10
// phase: P2.1
// tsc: accepts
// expected-error: C10: the referenced function reads a binding that is not declared yet
function run(go: boolean): void {
  if (go) {
    show();
  }
  const label = "x";
  function show(): void {
    print(label);
  }
}

export function main(): void {
  run(true);
}
