// corpus: reject/r03-optional-parameter
// purpose: Rejects an optional parameter without a default.
// rule: C2
// phase: P2.5
// tsc: accepts
// expected-error: C2: an optional parameter needs a default
function greet(name?: string): string {
  return name ?? "nobody";
}

export function main(): void {
  print(greet());
}
