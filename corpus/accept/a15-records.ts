// corpus: accept/a15-records
// purpose: Builds, reads, and updates object values with a nullable field.
// rule: C1, C2
// phase: P2.5
// tsc: accepts
// js-comparable: yes
type Point = { x: number; y: number };
type Tagged = { point: Point; tag: string | null };

function moved(p: Point, dx: number): Point {
  return { x: p.x + dx, y: p.y };
}

export function main(): void {
  const p: Point = { x: 1, y: 2 };
  const q = moved(p, 3);
  print(`${p.x} ${q.x} ${q.y}`);
  const t: Tagged = { point: q, tag: null };
  print(t.tag ?? "untagged");
  t.tag = "set";
  t.point.y = 7;
  print(`${t.tag ?? "untagged"} ${q.y}`);
}
