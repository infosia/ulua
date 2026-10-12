// corpus: reject/r104-narrowed-nullable-field
// purpose: Rejects a narrowed nullable reference as a field value of an object literal without a contextual type.
// rule: C11
// phase: P2.5
// tsc: errors TS2322
// expected-error: C11: an inferred type cannot come from a narrowable reference
type Box = { value: number };

export function main(): void {
  let y: Box | null = { value: 1 };
  const o = { b: y };
  o.b = null;
  print(`${o.b == null}`);
}
