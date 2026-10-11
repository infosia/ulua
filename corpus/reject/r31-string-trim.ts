// corpus: reject/r31-string-trim
// purpose: Rejects trim, which Luau patterns apply to ASCII whitespace only.
// rule: C5
// phase: P2
// tsc: accepts
// expected-error: C5: no lowering of trim agrees on non-ASCII input
export function main(): void {
  print("  x　".trim());
}
