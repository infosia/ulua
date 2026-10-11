// corpus: reject/r36-hoisted-function-as-value
// purpose: Rejects a hoisted function passed as a value before a binding that it reads.
// rule: C10
// tsc: accepts
// expected-error: C10: the referenced function reads a binding that is not declared yet
function apply(f: () => void): void {
  f();
}

export function main(): void {
  apply(show);
  const label = "ready";
  function show(): void {
    print(label);
  }
}
