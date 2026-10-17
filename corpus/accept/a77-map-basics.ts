// corpus: accept/a77-map-basics
// purpose: Map with number and string keys, NaN and -0 keys, get misses, getOr, has, delete, size, clear, and a copy.
// rule: C6
// phase: P7.4
// tsc: accepts
// js-comparable: yes
export function main(): void {
  const m = new Map<number, string>();
  m.set(1, "one").set(2, "two");
  m.set(NaN, "nan");
  m.set(-0, "zero");
  print(`${m.size} ${m.get(NaN) ?? "-"} ${m.get(0) ?? "-"} ${m.get(3) ?? "-"} ${m.has(2)} ${m.has(5)}`);
  const hit = m.get(1);
  if (hit != null) {
    print(`hit ${hit}`);
  }
  print(`${m.getOr(7, "seven")} ${m.getOr(1, "x")} ${m.delete(2)} ${m.delete(2)} ${m.size}`);
  const copy = new Map<number, string>(m);
  m.clear();
  print(`${m.size} ${copy.size} ${copy.get(1) ?? "-"}`);
  const s = new Map<string, number>();
  s.set("a", 1);
  s.set("a", s.getOr("a", 0) + 1);
  print(`${s.get("a") ?? -1} ${s.get("b") == null}`);
}
