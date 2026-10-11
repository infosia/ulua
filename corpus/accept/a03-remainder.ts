// corpus: accept/a03-remainder
// purpose: Lowers the remainder operator with the sign of the dividend.
// rule: C4
// tsc: accepts
function isNegativeZero(x: number): boolean {
  return x === 0 && 1 / x < 0;
}

export function main(): void {
  print(`${-7 % 3} ${7 % -3} ${5.5 % 2} ${5 % 0}`);
  print(`${2 % Infinity} ${-2 % Infinity} ${Infinity % 2}`);
  print(`${isNegativeZero(-4 % 2)}`);
  let x = 17;
  x %= 5;
  print(`${x}`);
}
