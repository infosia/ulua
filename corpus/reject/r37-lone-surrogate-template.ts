// corpus: reject/r37-lone-surrogate-template
// purpose: Rejects lone surrogates in a template and in a code point escape.
// rule: C5
// phase: P2.4
// tsc: accepts
// expected-error: C5: a string literal must not contain a lone surrogate
export function main(): void {
  const high = `\uD83D`;
  const low = "\u{DE00}";
  print(`${high + low === "😀"}`);
}
