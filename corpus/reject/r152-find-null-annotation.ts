// corpus: reject/r152-find-null-annotation
// purpose: Rejects a find result held in a T | null annotation; tsc types the result T | undefined.
// rule: C2
// phase: P7.2
// tsc: errors TS2322
// expected-error: C2: a find result is held only through an inferred type
export function main(): void {
  const xs: number[] = [1, 2, 3];
  const r: number | null = xs.find((v: number): boolean => v > 5);
  print(`${r ?? 0}`);
}
