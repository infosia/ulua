// corpus: accept/a86-json-mismatch
// purpose: Well-formed JSON text that does not match the target type throws TypeError, a field outside the target is dropped, and a parse past depth 128 throws SyntaxError; node keeps each value.
// rule: C20
// phase: P7.6
// tsc: accepts
// js-comparable: no C20
type Point = { x: number; y: number };
type Chain = { v: number; next: Chain | null };

function check(text: string): string {
  try {
    const p: Point = JSON.parse(text);
    return `ok ${p.x + p.y}`;
  } catch (e) {
    if (e instanceof TypeError) {
      return "type";
    }
    if (e instanceof SyntaxError) {
      return "syntax";
    }
    return "other";
  }
}

export function main(): void {
  print(`${check('{"x":1,"y":2}')} ${check('{"x":1}')} ${check('{"x":"1","y":2}')} ${check('{"x":null,"y":2}')}`);
  print(`${check("[1,2]")} ${check("3")} ${check('{"x":1e400,"y":0}')} ${check('{"x":1,"y":2')}`);
  const p: Point = JSON.parse('{"x":1,"y":2,"z":3}');
  print(JSON.stringify(p));
  let text = "null";
  for (let i = 0; i < 200; i++) {
    text = `{"v":${i},"next":${text}}`;
  }
  try {
    const c: Chain = JSON.parse(text);
    print(`${c.v}`);
  } catch (e) {
    print(e instanceof SyntaxError ? "syntax" : "other");
  }
}
