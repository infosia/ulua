// corpus: reject/r105-narrowed-boolean-field
// purpose: Rejects a narrowed boolean reference as a field value inside an array literal without a contextual type.
// rule: C11
// phase: P2.5
// tsc: errors TS2322
// expected-error: C11: an inferred type cannot come from a narrowable reference
function t(): boolean {
  return true;
}

export function main(): void {
  const b: boolean = t();
  if (b) {
    const os = [{ v: b }];
    os[0].v = false;
    print(`${os[0].v}`);
  }
}
