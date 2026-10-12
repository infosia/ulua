// corpus: accept/a46-contradicted-null-test
// purpose: Keeps a narrowing at a join with an edge where a null test contradicts it, and accepts a ?? test with a literal.
// rule: C16
// phase: P2.5
// tsc: accepts
// js-comparable: yes
type B = { v: number };

function f(p: B | null, bn: boolean | null): number {
  const x: B | null = p;
  let out = 0;
  if (x != null || x != null) {
    out += x.v;
  }
  if (x == null) {
    return out;
  }
  if (bn ?? true) {
    out += 10;
  }
  return out + x.v;
}

export function main(): void {
  print(`${f({ v: 2 }, null)} ${f({ v: 3 }, false)} ${f(null, true)}`);
}
