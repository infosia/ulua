// corpus: accept/a28-boolean-equality
// purpose: Compares narrowable boolean bindings with operands that tsc does not narrow to a literal type.
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
export function main(): void {
  print(`${f(false, true)} ${g(false)} ${h(2)}`);
}
