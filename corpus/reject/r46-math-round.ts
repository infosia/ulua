// corpus: reject/r46-math-round
// purpose: Rejects a library global outside the accepted surface.
// rule: C11
// tsc: accepts
// expected-error: C11: Math is not part of the accepted surface
export function main(): void {
  print(`${Math.round(-0.5)}`);
}
