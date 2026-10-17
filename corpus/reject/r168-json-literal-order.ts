// corpus: reject/r168-json-literal-order
// purpose: Rejects an object literal that reaches JSON.stringify with its fields out of declaration order (D34).
// rule: C20
// phase: P7.6
// tsc: accepts
// expected-error: C20: an object literal that reaches JSON.stringify lists its fields in declaration order
type Point = { x: number; y: number };

export function main(): void {
  const p: Point = { y: 2, x: 1 };
  print(JSON.stringify(p));
}
