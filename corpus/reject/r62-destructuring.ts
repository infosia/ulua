// corpus: reject/r62-destructuring
// purpose: Rejects a binding pattern with a default.
// rule: C11
// phase: P2.5
// tsc: accepts
// expected-error: C11: a binding is an identifier
type Tagged = { tag: string | null };

function show(t: Tagged): void {
  const { tag = "dflt" } = t;
  print(tag ?? "none");
}

export function main(): void {
  show({ tag: null });
}
