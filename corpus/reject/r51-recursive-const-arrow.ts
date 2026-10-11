// corpus: reject/r51-recursive-const-arrow
// purpose: Rejects a const arrow that refers to its own binding.
// rule: C10
// tsc: accepts
// expected-error: C10: a binding is used before the end of its declaration
export function main(): void {
  const count = (n: number): number => (n <= 0 ? 0 : 1 + count(n - 1));
  print(`${count(3)}`);
}
