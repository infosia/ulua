// corpus: reject/r32-string-position-argument
// purpose: Rejects the position argument of startsWith.
// rule: C5
// phase: P2.4
// tsc: accepts
// expected-error: C5: a string position depends on the encoding
export function main(): void {
  print(`${"héllo".startsWith("l", 2)}`);
}
