// corpus: reject/r61-object-literal-method
// purpose: Rejects a method entry and a spread in object literals.
// rule: C11
// phase: P2
// tsc: accepts
// expected-error: C11: an object literal holds key: expression entries only
type F = { f: () => number };

export function main(): void {
  const a: F = { f(): number { return 3; } };
  const b: F = { ...a };
  print(`${b.f()}`);
}
