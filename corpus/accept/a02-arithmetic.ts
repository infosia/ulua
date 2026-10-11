// corpus: accept/a02-arithmetic
// purpose: Evaluates the four arithmetic operators with precedence and grouping.
// rule: C4
// phase: P1
// tsc: accepts
export function main(): void {
  const a = 7;
  const b = 2;
  print(`${a + b} ${a - b} ${a * b} ${a / b}`);
  print(`${a + b * 3} ${(a + b) * 3} ${a - b - 1} ${a / b / 2}`);
  print(`${-a} ${0.1 + 0.2} ${1 / 3}`);
}
