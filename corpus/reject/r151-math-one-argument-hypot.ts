// corpus: reject/r151-math-one-argument-hypot
// purpose: Rejects Math.hypot with one argument; the accepted arity is two.
// rule: C4
// phase: P7
// tsc: accepts
// expected-error: C4: Math.hypot takes exactly two arguments
export function main(): void {
  print(`${Math.hypot(3)}`);
}
