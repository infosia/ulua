// corpus: reject/r08-string-length
// purpose: Rejects the UTF-16 length of a string.
// rule: C5
// phase: P2.4
// tsc: accepts
// expected-error: C5: string length depends on the encoding
export function main(): void {
  const s = "héllo";
  print(`${s.length}`);
}
