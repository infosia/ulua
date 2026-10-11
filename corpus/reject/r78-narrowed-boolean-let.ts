// corpus: reject/r78-narrowed-boolean-let
// purpose: Rejects an unannotated boolean let whose initializer is a narrowed reference.
// rule: C11
// phase: P1
// tsc: errors TS2322
// expected-error: C11: an unannotated boolean let needs a literal or comparison initializer
function flip(b: boolean): boolean {
  if (b) {
    let v = b;
    v = false;
    return v;
  }
  return b;
}

export function main(): void {
  print(`${flip(true)}`);
}
