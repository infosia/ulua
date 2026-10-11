// corpus: reject/r10-string-ordering
// purpose: Rejects an ordering comparison between strings.
// rule: C5
// phase: P2
// tsc: accepts
// expected-error: C5: string ordering depends on the encoding
export function main(): void {
  const a = "￿";
  const b = "😀";
  print(`${a < b}`);
}
