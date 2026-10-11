// corpus: accept/a25-template-line-ends
// purpose: Changes a raw CR LF and a raw CR in a template literal to LF, as JavaScript does.
// rule: C14
// phase: P1
// tsc: accepts
// js-comparable: yes
export function main(): void {
  print(`a
bc`);
  const n = 2;
  print(`${n}
${n + 1}`);
}
