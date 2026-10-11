// corpus: accept/a30-function-parameter-default
// purpose: Uses a parameter of function type with a default, called with and without the argument.
// rule: C11
// phase: P2.1
// tsc: accepts
// js-comparable: yes
function test(n: number, f: (a: number) => boolean = (a: number): boolean => a > 1): boolean {
  return f(n);
}

export function main(): void {
  print(`${test(2)} ${test(0)} ${test(0, (a: number): boolean => a === 0)}`);
}
