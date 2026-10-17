// corpus: accept/a84-json-parse
// purpose: JSON.parse into a declared target type: records, arrays, nested values, null, extra fields, duplicate keys, -0, escapes, and a round trip.
// rule: C20
// phase: P7.6
// tsc: accepts
// js-comparable: yes
type Point = { x: number; y: number };
type Doc = { title: string; tags: string[]; origin: Point; note: string | null; done: boolean };

function total(points: Point[]): number {
  let sum = 0;
  for (const p of points) {
    sum = sum + p.x + p.y;
  }
  return sum;
}

export function main(): void {
  const p: Point = JSON.parse(' { "y" : 2 , "x" : 1 , "extra" : [1, {"z": 2}] } ');
  print(`${p.x} ${p.y}`);
  const xs: number[] = JSON.parse("[1, -2.5, 3e2, 0.125]");
  print(`${xs.length} ${xs[0] + xs[1] + xs[2] + xs[3]}`);
  const d: Doc = JSON.parse('{"title":"A\\u0042\\n\\"c\\"","tags":["x","\\ud83d\\ude00"],"origin":{"x":0,"y":5},"note":null,"done":true}');
  print(`${d.title} ${d.tags.length} ${d.tags[1]} ${d.origin.y} ${d.note ?? "none"} ${d.done}`);
  const q: Point = JSON.parse('{"x":1,"y":2,"x":10}');
  print(`${q.x} ${q.y}`);
  const z: number = JSON.parse("-0");
  print(`${1 / z}`);
  print(`${total(JSON.parse('[{"x":1,"y":2},{"x":3,"y":4}]'))}`);
  const back: Doc = JSON.parse(JSON.stringify(d));
  print(JSON.stringify(back));
  const s: string = JSON.parse('"\\u00e9\\t"');
  print(`${s === "é\t"}`);
}
