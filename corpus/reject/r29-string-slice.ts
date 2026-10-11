// corpus: reject/r29-string-slice
// purpose: Rejects slice on a string.
// rule: C5
// phase: P2.3
// tsc: accepts
// expected-error: C5: slice depends on the encoding
export function main(): void {
  print("héllo".slice(0, 2));
}
