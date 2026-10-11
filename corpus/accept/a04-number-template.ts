// corpus: accept/a04-number-template
// purpose: Formats special and boundary numbers in template literals.
// rule: C4
// phase: P1
// tsc: accepts
// js-comparable: yes
export function main(): void {
  const zero = 0;
  print(`${-zero} ${1e-6} ${1e-7} ${1.5e-7}`);
  print(`${0 / zero} ${1 / zero} ${-1 / zero}`);
  print(`${1e20} ${1e21} ${9007199254740993}`);
  print(`${5e-324} ${1.7976931348623157e308} ${String(100)}`);
}
