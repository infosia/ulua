// corpus: accept/a49-equal-aliases-through-links
// purpose: Relates two equal record aliases through arrays, nullable types, and function types (D19).
// rule: C1
// phase: P2.5
// tsc: accepts
// js-comparable: yes
type P = { v: number };
type Q = { v: number };
type R = { xs: Q[]; x: Q | null; grid: Q[][]; f: (a: P, b: Q) => number; g: (a: P) => Q };
type A = { n: B | null; v: number };
type B = { n: A | null; v: number };

function first(a: A): B | null {
  return a.n;
}

export function main(): void {
  const r: R = {
    xs: [{ v: 1 }],
    x: { v: 2 },
    grid: [[{ v: 3 }]],
    f: (a: P, b: Q): number => a.v + b.v,
    g: (a: P): Q => a,
  };
  const ys: P[] = r.xs;
  const y: P | null = r.x;
  const zs: P[][] = r.grid;
  const h: (a: Q, b: P) => number = r.f;
  const k: (a: Q) => P = r.g;
  const a: A = { n: { n: null, v: 5 }, v: 4 };
  const b: B | null = first(a);
  print(`${ys.length} ${y != null ? y.v : 0} ${zs[0][0].v} ${h({ v: 1 }, { v: 2 })} ${k({ v: 7 }).v} ${b != null ? b.v : 0}`);
}
