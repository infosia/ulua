// corpus: accept/a27-parameter-callee
// purpose: Calls a parameter of function type, with a declaration, a binding, and an arrow as the argument.
// rule: C11
// phase: P2
// tsc: accepts
function twice(f: (n: number) => number, n: number): number {
  return f(f(n));
}

function inc(n: number): number {
  return n + 1;
}

export function main(): void {
  const double = (n: number): number => n * 2;
  print(`${twice(inc, 1)}`);
  print(`${twice(double, 3)}`);
  print(`${twice((n: number): number => n - 5, 0)}`);
}
