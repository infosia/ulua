// corpus: accept/a43-const-narrowing-arrow
// purpose: Keeps the null narrowing of a const inside an arrow body (C16.4).
// rule: C16
// phase: P2.5
// tsc: accepts
// js-comparable: yes
type Box = { value: number };

function read(p: Box | null): number {
  const x: Box | null = p;
  if (x === null) {
    return 0;
  }
  const twice = (): number => x.value * 2;
  return twice();
}

export function main(): void {
  print(`${read({ value: 3 })} ${read(null)}`);
}
