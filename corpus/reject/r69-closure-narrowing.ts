// corpus: reject/r69-closure-narrowing
// purpose: Rejects a narrowed let binding inside an arrow body that runs after a later assignment.
// rule: C16
// tsc: accepts
// expected-error: C16: the narrowing of a let binding does not continue into a function body
export function main(): void {
  let s: string | null = "a";
  if (s != null) {
    const f = (): string => s + "x";
    s = null;
    print(f());
  }
}
