// corpus: accept/a28-boolean-equality
// purpose: Compares operands whose literal types overlap or that tsc does not narrow to a literal type.
// rule: C7
// phase: P1
// tsc: accepts
function f(p: boolean, q: boolean): boolean {
  p = true;
  return p === q;
}
function g(p: boolean): boolean {
  return p === true;
}
function h(n: number): boolean {
  let x = n > 1;
  return x === g(true);
}
function k(c: boolean, p: boolean): string {
  return `${(p && true) === false} ${(c ? 1 : 2) === 2} ${(!p) === true} ${(!true) === false}`;
}
export function main(): void {
  print(`${f(false, true)} ${g(false)} ${h(2)}`);
  print(k(true, false));
}
