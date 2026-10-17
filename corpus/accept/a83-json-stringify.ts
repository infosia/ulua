// corpus: accept/a83-json-stringify
// purpose: JSON.stringify of numbers, strings with escapes, booleans, null, arrays, records, nested records, and class fields in declaration order.
// rule: C20
// phase: P7.6
// tsc: accepts
// js-comparable: yes
type Point = { x: number; y: number };
type Shape = { name: string; points: Point[]; closed: boolean; tag: string | null };
class Item {
  id: number;
  label: string;
  constructor(label: string, id: number) {
    this.label = label;
    this.id = id;
  }
}

export function main(): void {
  print(`${JSON.stringify(1.5)} ${JSON.stringify(-0)} ${JSON.stringify(1e21)} ${JSON.stringify(0.1 + 0.2)}`);
  print(JSON.stringify("q\"b\\s/\b\t\n\f\r\u0001\u001f\u007f é😀"));
  print(`${JSON.stringify(true)} ${JSON.stringify(false)}`);
  const empty: number[] = [];
  print(`${JSON.stringify(empty)} ${JSON.stringify([1, 2, 3])} ${JSON.stringify(["a", "b"])}`);
  const p: Point = { x: 1, y: -2 };
  print(JSON.stringify(p));
  const s: Shape = { name: "tri", points: [{ x: 0, y: 0 }, { x: 1, y: 0 }, { x: 0, y: 1 }], closed: true, tag: null };
  print(JSON.stringify(s));
  const t: Shape = { name: "line", points: [], closed: false, tag: "t" };
  print(JSON.stringify(t));
  print(JSON.stringify(new Item("box", 7)));
  const grid: number[][] = [[1], [], [2, 3]];
  print(JSON.stringify(grid));
  const maybe: string | null = null;
  print(JSON.stringify(maybe));
}
