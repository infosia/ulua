// corpus: accept/a86-json-mismatch
// purpose: Well-formed JSON text that does not match the target type throws TypeError; node returns the value.
// rule: C20
// phase: P7.6
// tsc: accepts
// js-comparable: no C20
type Point = { x: number; y: number };

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
}
