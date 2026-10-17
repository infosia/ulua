// corpus: reject/r164-locale-upper
// purpose: Rejects toLocaleUpperCase, which depends on locale data.
// rule: C5
// phase: P7.5
// tsc: accepts
// expected-error: C5: toLocaleUpperCase depends on locale data
export function main(): void {
  print("i".toLocaleUpperCase());
}
