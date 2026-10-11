// corpus: reject/r87-inferred-function-result
// purpose: Rejects an arrow whose inferred result is a function type.
// rule: C11
// phase: P2.1
// tsc: accepts
// expected-error: C11: an arrow with a function result needs a result type annotation
export function main(): void {
  const make = () => (): number => 1;
  print(`${make()()}`);
}
