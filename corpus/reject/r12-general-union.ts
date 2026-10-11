// corpus: reject/r12-general-union
// purpose: Rejects a union that is not T | null.
// rule: C2
// phase: P1
// tsc: accepts
// expected-error: C2: the only union is T | null
function show(v: number | string): string {
  return `${v}`;
}

export function main(): void {
  print(show(1));
}
