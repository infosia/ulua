// corpus: accept/a69-math-exact
// purpose: Math members that equal node on every input: builtin aliases, helpers, edge rules, and constants.
// rule: C4
// phase: P7
// tsc: accepts
// js-comparable: yes
function z(x: number): string {
  return `${x} ${1 / x}`;
}

export function main(): void {
  print(`${Math.abs(-3.5)} ${z(Math.abs(-0))} ${Math.ceil(1.2)} ${z(Math.ceil(-0.5))} ${Math.floor(-1.5)} ${Math.sqrt(2)} ${Math.sqrt(-1)}`);
  print(`${Math.round(2.5)} ${Math.round(-2.5)} ${z(Math.round(-0.5))} ${z(Math.round(-0.4))} ${Math.round(0.49999999999999994)} ${Math.round(1.5)} ${Math.round(-1.5)}`);
  print(`${Math.sign(-3)} ${Math.sign(7)} ${z(Math.sign(-0))} ${Math.sign(NaN)} ${z(Math.trunc(-0.7))} ${Math.trunc(2.9)} ${Math.trunc(-Infinity)}`);
  print(`${Math.max(1, NaN)} ${Math.max(NaN, 1)} ${z(Math.max(-0, 0))} ${z(Math.max(0, -0))} ${z(Math.min(0, -0))} ${Math.min(2, 3)} ${Math.max(-Infinity, -1)}`);
  print(`${Math.pow(1, NaN)} ${Math.pow(-1, Infinity)} ${Math.pow(NaN, 0)} ${Math.pow(2, 10)} ${Math.pow(4, 0.5)} ${Math.pow(0, -1)}`);
  print(`${Math.clz32(0)} ${Math.clz32(1)} ${Math.clz32(-1)} ${Math.clz32(4294967296)} ${Math.clz32(0.5)} ${Math.clz32(NaN)}`);
  print(`${Math.imul(3, 4)} ${Math.imul(-5, 12)} ${Math.imul(4294967295, 5)} ${Math.imul(65536, 65536)} ${Math.imul(2147483647, 2)}`);
  print(`${Math.fround(5.5)} ${Math.fround(5.05)} ${Math.fround(1e40)} ${z(Math.fround(-0))} ${Math.fround(1.0000001192092896)}`);
  print(`${Math.E} ${Math.LN2} ${Math.LN10} ${Math.LOG2E} ${Math.LOG10E} ${Math.PI} ${Math.SQRT1_2} ${Math.SQRT2}`);
  let n = -3;
  let total = 0;
  while (n <= 3) {
    total = total + Math.abs(n) * Math.sign(n) + Math.round(n / 2) + Math.trunc(n / 2);
    n = n + 0.5;
  }
  print(`${total}`);
}
