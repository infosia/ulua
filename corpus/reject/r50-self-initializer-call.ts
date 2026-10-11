// corpus: reject/r50-self-initializer-call
// purpose: Rejects a call in an initializer of a function that reads the binding being declared.
// rule: C10
// tsc: accepts
// expected-error: C10: the referenced function reads a binding that is not declared yet
const limit: number = cap();

function cap(): number {
  return limit + 1;
}

export function main(): void {
  print(`${limit}`);
}
