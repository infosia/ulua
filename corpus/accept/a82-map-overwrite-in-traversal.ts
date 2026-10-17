// corpus: accept/a82-map-overwrite-in-traversal
// purpose: A set on a present key or an add of a present member during a traversal changes no entry and does not trap.
// rule: C6
// phase: P7.4
// tsc: accepts
// js-comparable: yes
export function main(): void {
  const m = new Map<string, number>();
  m.set("a", 1).set("b", 2).set("c", 3);
  m.forEach((v: number, k: string): void => {
    m.set(k, v * 10);
  });
  for (const k of m.keys()) {
    m.set(k, m.getOr(k, 0) + 1);
  }
  let line = "";
  for (const v of m.values()) {
    line += `${v} `;
  }
  print(`${line}${m.size}`);
  const s = new Set<number>([1, 2, 3]);
  for (const k of s) {
    s.add(k);
  }
  s.forEach((k: number): void => {
    s.add(1);
  });
  print(`${s.size}`);
}
