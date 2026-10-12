// corpus: reject/r119-throw-non-error
// purpose: Rejects a throw of a value outside the Error family.
// rule: C17
// phase: P2.7
// tsc: accepts
// expected-error: C17: a thrown value has the Error type
export function main(): void {
  try {
    throw 42;
  } catch {
    print("caught");
  }
}
