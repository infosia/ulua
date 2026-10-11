// corpus: reject/r26-typeof
// purpose: Rejects the typeof operator.
// rule: C8
// tsc: accepts
// expected-error: C8: typeof has no lowering
export function main(): void {
  const n = 1;
  print(typeof n);
}
