// corpus: reject/r05-strict-null-equality
// purpose: Rejects a strict comparison with null.
// rule: C2
// phase: P2.4
// tsc: accepts
// expected-error: C2: test null with == null or != null
function first(xs: string[]): string | null {
  return xs.length > 0 ? xs[0] : null;
}

export function main(): void {
  const s = first([]);
  print(`${s === null}`);
}
