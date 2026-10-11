// corpus: reject/r85-non-trailing-default
// purpose: Rejects a parameter with a default before a parameter without one.
// rule: C11
// phase: P2.1
// tsc: accepts
// expected-error: C11: a parameter with a default needs defaults on each later parameter
function g(a: number = 1, b: number): number {
  return a * b;
}

export function main(): void {
  print(`${g(2, 3)}`);
}
