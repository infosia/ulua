// corpus: reject/r120-catch-annotation
// purpose: Rejects a catch binding annotated with a type other than unknown.
// rule: C17
// phase: P2.7
// tsc: errors TS1196
// expected-error: C17: a catch binding has no annotation other than unknown
export function main(): void {
  try {
    throw new Error("m");
  } catch (e: Error) {
    print(e.message);
  }
}
