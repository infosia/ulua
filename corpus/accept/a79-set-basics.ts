// corpus: accept/a79-set-basics
// purpose: Set from an array, add, has, delete, clear, for...of order, and identity keys of a class.
// rule: C6
// phase: P7.4
// tsc: accepts
// js-comparable: yes
class Node {
  id: number;
  constructor(id: number) {
    this.id = id;
  }
}

export function main(): void {
  const s = new Set<number>([3, 1, 3, 2, NaN, NaN, -0, 0]);
  let line = "";
  for (const k of s) {
    line = `${line} ${k}`;
  }
  print(`${s.size} |${line}`);
  s.add(1).add(9);
  print(`${s.has(9)} ${s.has(4)} ${s.delete(3)} ${s.delete(3)} ${s.size}`);
  const a = new Node(1);
  const b = new Node(1);
  const nodes = new Set<Node>();
  nodes.add(a);
  nodes.add(b);
  nodes.add(a);
  print(`${nodes.size} ${nodes.has(a)} ${nodes.has(new Node(1))}`);
  let ids = 0;
  nodes.forEach((n: Node): void => {
    ids = ids + n.id;
  });
  const copy = new Set<number>(s);
  s.clear();
  print(`${ids} ${s.size} ${copy.size}`);
}
