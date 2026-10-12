// corpus: reject/r97-narrowed-let-copy
// purpose: Rejects an unannotated let whose initializer is a reference; tsc takes its narrowed type.
// rule: C11
// phase: P2.3
// tsc: errors TS2322, TS2367
// expected-error: C11: an inferred type cannot come from a reference
export function main(): void {
  const s = "a";
  if (s === "b") {
    let t = s;
    t = "c";
    print(t);
  }
  print(s);
}
