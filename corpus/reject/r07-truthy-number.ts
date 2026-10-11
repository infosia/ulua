// corpus: reject/r07-truthy-number
// purpose: Rejects a truth test on a number.
// rule: C3
// tsc: accepts
// expected-error: C3: a condition must be boolean
export function main(): void {
  const n = 0;
  if (n) {
    print("nonzero");
  }
}
