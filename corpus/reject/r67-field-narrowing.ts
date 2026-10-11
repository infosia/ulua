// corpus: reject/r67-field-narrowing
// purpose: Rejects a narrowed field read after a call that can clear the field.
// rule: C16
// phase: P2
// tsc: accepts
// expected-error: C16: a field is never narrowed; copy it into a const
type T = { tag: string | null };

const t: T = { tag: "a" };

function clear(): void {
  t.tag = null;
}

export function main(): void {
  if (t.tag != null) {
    clear();
    print(t.tag + "x");
  }
}
