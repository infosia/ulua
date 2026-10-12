// corpus: reject/r100-narrowed-nullable-copy
// purpose: Rejects an unannotated let that copies a narrowed nullable reference; tsc types it without null.
// rule: C11
// phase: P2.5
// tsc: errors TS2322
// expected-error: C11: an inferred type cannot come from a narrowable reference
type Box = { value: number };

function reset(x: Box | null): number {
  if (x != null) {
    let y = x;
    y = null;
    return y == null ? 0 : 1;
  }
  return 2;
}

export function main(): void {
  print(`${reset({ value: 1 })}`);
}
