// corpus: reject/r44-template-null
// purpose: Rejects template substitutions of a nullable value, an object, and an array.
// rule: C4
// phase: P2.5
// tsc: accepts
// expected-error: C4: a template substitution must be a number, string, or boolean
type Point = { x: number };

function show(s: string | null, p: Point, xs: number[]): void {
  print(`${s} ${p} ${xs}`);
}

export function main(): void {
  show(null, { x: 1 }, [1, 2]);
}
