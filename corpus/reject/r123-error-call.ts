// corpus: reject/r123-error-call
// purpose: Rejects an Error class called without new.
// rule: C17
// phase: P2.7
// tsc: accepts
// expected-error: C17: an Error class is called only with new
export function main(): void {
  const e = Error("m");
  print(e.message);
}
