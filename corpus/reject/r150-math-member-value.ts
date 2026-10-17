// corpus: reject/r150-math-member-value
// purpose: Rejects a Math member read as a value.
// rule: C4
// phase: P7
// tsc: accepts
// expected-error: C4: a Math member is called or read as a constant
export function main(): void {
  const f: (x: number) => number = Math.sin;
  print(`${f(0)}`);
}
