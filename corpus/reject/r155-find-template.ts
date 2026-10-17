// corpus: reject/r155-find-template
// purpose: Rejects a find result as a template substitution; it prints undefined under node on a miss.
// rule: C2
// phase: P7.2
// tsc: accepts
// expected-error: C2: a find result is not a template substitution
export function main(): void {
  const xs: number[] = [1];
  const hit = xs.find((v: number): boolean => v > 5);
  print(`${hit}`);
}
