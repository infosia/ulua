// corpus: trap/t09-trap-not-caught
// purpose: A trap stops the run; no catch receives it and no finally runs.
// rule: C17
// phase: P2.7
// tsc: accepts
// js-comparable: no C17
// trap: invalid-argument
export function main(): void {
  print("start");
  try {
    print("x".repeat(-1));
  } catch {
    print("caught");
  } finally {
    print("finally");
  }
  print("after");
}
