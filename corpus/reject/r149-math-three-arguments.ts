// corpus: reject/r149-math-three-arguments
// purpose: Rejects Math.max with three arguments; the accepted arity is two.
// rule: C4
// phase: P7
// tsc: accepts
// expected-error: C4: Math.max takes exactly two arguments
export function main(): void {
  print(`${Math.max(1, 2, 3)}`);
}
