// corpus: reject/r109-contradicting-null-test
// purpose: Rejects a read after a join where a null test that contradicts a narrowing keeps its edge.
// rule: C16
// phase: P2.5
// tsc: errors TS18047
// expected-error: C16: a null test never ends an edge
type B = { v: number };

function mk(): B | null {
  return { v: 2 };
}

export function main(): void {
  let out = 0;
  const k: B | null = mk();
  const c2: B | null = mk();
  if (k != null) {
    if (c2 != null) {
    } else if (k != null) {
      return;
    }
    out += c2.v;
  }
  print(`${out}`);
}
