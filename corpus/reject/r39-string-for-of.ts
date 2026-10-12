// corpus: reject/r39-string-for-of
// purpose: Rejects for-of over a string.
// rule: C5
// phase: P2.4
// tsc: accepts
// expected-error: C5: iteration over a string depends on the encoding
export function main(): void {
  for (const ch of "hé") {
    print(ch);
  }
}
