// corpus: accept/a85-json-errors
// purpose: Malformed JSON text throws SyntaxError and a cycle in JSON.stringify throws TypeError; both are caught.
// rule: C20
// phase: P7.6
// tsc: accepts
// js-comparable: yes
type Point = { x: number; y: number };
class Link {
  value: number;
  next: Link | null;
  constructor(value: number) {
    this.value = value;
    this.next = null;
  }
}

function tryParse(text: string): string {
  try {
    const p: Point = JSON.parse(text);
    return `ok ${p.x}`;
  } catch (e) {
    if (e instanceof SyntaxError) {
      return "syntax";
    }
    return "other";
  }
}

export function main(): void {
  print(`${tryParse('{"x":1,"y":2}')} ${tryParse("")} ${tryParse("{")} ${tryParse('{"x":1,}')}`);
  print(`${tryParse("{'x':1}")} ${tryParse('{"x":01,"y":2}')} ${tryParse('{"x":1,"y":2} x')} ${tryParse('{"x":NaN}')}`);
  const a = new Link(1);
  const b = new Link(2);
  a.next = b;
  print(JSON.stringify(a));
  b.next = a;
  try {
    print(JSON.stringify(a));
  } catch (e) {
    print(e instanceof TypeError ? "cycle" : "other");
  }
}
