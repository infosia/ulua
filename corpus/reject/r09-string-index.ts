// corpus: reject/r09-string-index
// purpose: Rejects an index into a string.
// rule: C5
// tsc: accepts
// expected-error: C5: a string index depends on the encoding
export function main(): void {
  const s = "héllo";
  print(s[1]);
}
