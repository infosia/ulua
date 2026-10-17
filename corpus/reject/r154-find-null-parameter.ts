// corpus: reject/r154-find-null-parameter
// purpose: Rejects a find result passed to a T | null parameter.
// rule: C2
// phase: P7.2
// tsc: errors TS2345
// expected-error: C2: a find result is held only through an inferred type
function take(x: number | null): number {
  return x ?? 0;
}

export function main(): void {
  const xs: number[] = [1, 2, 3];
  print(`${take(xs.find((v: number): boolean => v > 5))}`);
}
