// corpus: reject/r34-lone-surrogate
// purpose: Rejects a string literal with an unpaired surrogate escape.
// rule: C5
// phase: P2.4
// tsc: accepts
// expected-error: C5: a string literal must not contain a lone surrogate
export function main(): void {
  const high = "\uD83D";
  print(`${high + "\uDE00" === "😀"}`);
}
