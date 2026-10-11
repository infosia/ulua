// corpus: reject/r84-conditional-function-defaults
// purpose: Rejects a conditional between function values with different numbers of defaults.
// rule: C1
// phase: P2.1
// tsc: errors TS2554
// expected-error: C1: function types differ in their defaults
function g(a: number = 7): number {
  return a;
}

export function main(): void {
  const f = (a: number): number => a * 2;
  const b = f(1) > 5 ? g : f;
  print(`${b()}`);
}
