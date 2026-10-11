// corpus: reject/r33-split-non-literal
// purpose: Rejects split with a separator that is not a non-empty literal.
// rule: C5
// phase: P2
// tsc: accepts
// expected-error: C5: the split separator must be a non-empty string literal
function pieces(s: string, sep: string): string[] {
  return s.split(sep);
}

export function main(): void {
  for (const p of pieces("hé", "")) {
    print(`[${p}]`);
  }
}
