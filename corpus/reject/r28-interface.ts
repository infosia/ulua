// corpus: reject/r28-interface
// purpose: Rejects an interface declaration.
// rule: C8
// phase: P2
// tsc: accepts
// expected-error: C8: interface has no lowering
interface Point {
  x: number;
}

export function main(): void {
  const p: Point = { x: 1 };
  print(`${p.x}`);
}
