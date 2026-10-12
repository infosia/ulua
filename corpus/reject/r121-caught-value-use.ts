// corpus: reject/r121-caught-value-use
// purpose: Rejects a member read of a catch binding without an instanceof test.
// rule: C17
// phase: P2.7
// tsc: errors TS18046
// expected-error: C17: a catch binding is only tested with instanceof or thrown
export function main(): void {
  try {
    throw new Error("m");
  } catch (e) {
    print(e.message);
  }
}
