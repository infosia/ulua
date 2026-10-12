// corpus: reject/r102-proto-field
// purpose: Rejects a field named __proto__; node reads the prototype object instead of the field.
// rule: C1
// phase: P2.5
// tsc: accepts
// expected-error: C1: a field named __proto__ is rejected
type T = { __proto__: number };

export function main(): void {
  const t: T = { __proto__: 1 };
  print(`${t.__proto__}`);
}
