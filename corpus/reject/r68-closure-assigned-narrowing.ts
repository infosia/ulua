// corpus: reject/r68-closure-assigned-narrowing
// purpose: Rejects narrowing of a let binding that a closure assigns.
// rule: C16
// phase: P2.4
// tsc: accepts
// expected-error: C16: a binding that a function assigns is never narrowed
export function main(): void {
  let s: string | null = "a";
  const clear = (): void => {
    s = null;
  };
  if (s != null) {
    clear();
    print(s + "x");
  }
}
