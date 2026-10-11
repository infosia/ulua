// corpus: reject/r18-to-upper-case
// purpose: Rejects case mapping that Luau applies to ASCII only.
// rule: C5
// phase: P2.3
// tsc: accepts
// expected-error: C5: no lowering of toUpperCase agrees on non-ASCII input
export function main(): void {
  print("é".toUpperCase());
}
