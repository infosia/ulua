// corpus: reject/r57-unary-minus-string
// purpose: Rejects unary minus on a string and on a boolean.
// rule: C4
// phase: P2.4
// tsc: accepts
// expected-error: C4: unary - takes a number
function negate(s: string, b: boolean): string {
  return `${-s} ${-b}`;
}

export function main(): void {
  print(negate("abc", true));
}
