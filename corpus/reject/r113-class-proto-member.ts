// corpus: reject/r113-class-proto-member
// purpose: Rejects a class field named __proto__.
// rule: C1
// phase: P2.6
// tsc: accepts
// expected-error: C1: a field named __proto__ is rejected
class P {
  __proto__: number = 1;
}

export function main(): void {
  const p = new P();
  print(`${p.__proto__}`);
}
