// corpus: reject/r42-closure-in-for-initializer
// purpose: Rejects a closure in the initializer of a for head.
// rule: C9
// phase: P2
// tsc: accepts
// expected-error: C9: a closure in a for initializer is not part of the language
export function main(): void {
  for (let i = 0, f = (): number => i; i < 3; i++) {
    print(`${f()}`);
  }
}
