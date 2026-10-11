// corpus: accept/a10-boolean-logic
// purpose: Combines boolean operators and compares with NaN.
// rule: C3, C4, C7
// phase: P1
// tsc: accepts
function logic(t: boolean, f: boolean): string {
  return `${t && f} ${t || f} ${!t} ${t ? f : !f} ${t ? !f : f}`;
}

export function main(): void {
  print(logic(true, false));
  let zero = 0;
  const nan = zero / zero;
  print(`${nan < 1} ${nan >= 1} ${!(nan < 1)} ${nan === nan} ${nan !== nan}`);
  print(`${zero === -zero} ${1 <= 1} ${2 > 1}`);
  const c = zero === 0;
  print(`${c ? false : true} ${!c ? true : false}`);
}
