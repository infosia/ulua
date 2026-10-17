// corpus: reject/r156-number-coerce
// purpose: Rejects Number(s), which converts its argument.
// rule: C4
// phase: P7.3
// tsc: accepts
// expected-error: C4: Number converts its argument; use parseFloat
export function main(): void {
  print(`${Number("5")}`);
}
