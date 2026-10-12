// corpus: accept/a47-nested-body-null-test
// purpose: Narrows a parameter and a let of the enclosing function by a null test inside an arrow body.
// rule: C16
// phase: P2.5
// tsc: accepts
// js-comparable: yes
type O = { v: number; o: O | null };

function f(b: O | null): number {
  let d: O | null = b != null ? b.o : null;
  const g = (c: O): number => c.v + (b != null ? b.v : 0) + (d != null ? d.v : 0);
  return g({ v: 1, o: null });
}

export function main(): void {
  print(`${f(null)} ${f({ v: 5, o: null })} ${f({ v: 5, o: { v: 7, o: null } })}`);
}
