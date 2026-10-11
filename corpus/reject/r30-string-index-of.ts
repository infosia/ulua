// corpus: reject/r30-string-index-of
// purpose: Rejects indexOf on a string.
// rule: C5
// phase: P2
// tsc: accepts
// expected-error: C5: indexOf depends on the encoding
export function main(): void {
  print(`${"héllo".indexOf("l")}`);
}
