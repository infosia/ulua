// corpus: trap/t14-from-code-point-range
// purpose: Traps on String.fromCodePoint above 0x10FFFF; node throws RangeError.
// rule: C5
// phase: P7.5
// tsc: accepts
// js-comparable: no C5
// trap: invalid-argument
export function main(): void {
  print(String.fromCodePoint(0x10FFFF) === String.fromCodePoint(1114111) ? "max" : "?");
  print(String.fromCodePoint(0x110000));
}
