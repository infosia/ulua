// corpus: reject/r158-to-string-no-radix
// purpose: Rejects number toString without a radix.
// rule: C4
// phase: P7.3
// tsc: accepts
// expected-error: C4: toString takes a radix
export function main(): void {
  const n: number = 5;
  print(n.toString());
}
