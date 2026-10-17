// corpus: trap/t15-from-char-code-surrogate
// purpose: Traps on String.fromCharCode of a surrogate; node gives a lone surrogate.
// rule: C5
// phase: P7.5
// tsc: accepts
// js-comparable: no C5
// trap: invalid-argument
export function main(): void {
  print(String.fromCharCode(0xD7FF) === String.fromCodePoint(0xD7FF) ? "below" : "?");
  print(String.fromCharCode(0xD800));
}
