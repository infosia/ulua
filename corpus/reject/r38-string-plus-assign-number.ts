// corpus: reject/r38-string-plus-assign-number
// purpose: Rejects += of a number onto a string.
// rule: C4
// phase: P2.4
// tsc: accepts
// expected-error: C4: + between a string and a non-string operand
export function main(): void {
  let s = "a";
  s += 1e-7;
  print(s);
}
