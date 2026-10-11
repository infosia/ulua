// corpus: reject/r74-template-equality
// purpose: Rejects an equality between a template and a literal; tsc gives the template a literal type.
// rule: C7
// phase: P1
// tsc: errors TS2367
// expected-error: C7: an equality needs a wide operand
export function main(): void {
  print(`${`${1}` === "2"}`);
}
