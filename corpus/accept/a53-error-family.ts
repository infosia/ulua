// corpus: accept/a53-error-family
// purpose: Constructs each Error class and tests class identity apart from the name field.
// rule: C17
// phase: P2.7
// tsc: accepts
// js-comparable: yes
function kinds(e: Error): string {
  return `${e instanceof Error} ${e instanceof TypeError} ${e instanceof RangeError} ${e instanceof SyntaxError} ${e instanceof ReferenceError} ${e instanceof EvalError} ${e instanceof URIError}`;
}

export function main(): void {
  const es: Error[] = [
    new Error("a"),
    new TypeError("b"),
    new RangeError("c"),
    new SyntaxError("d"),
    new ReferenceError("e"),
    new EvalError("f"),
    new URIError("g"),
  ];
  for (const e of es) {
    print(`${e.name}:${e.message} ${kinds(e)}`);
  }
  const blank = new Error();
  print(`[${blank.message}] ${blank.message.length}`);
  const t: TypeError = new RangeError("moved");
  t.name = "custom";
  print(`${t.name} ${t instanceof RangeError} ${t instanceof TypeError}`);
}
