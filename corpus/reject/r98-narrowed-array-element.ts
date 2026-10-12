// corpus: reject/r98-narrowed-array-element
// purpose: Rejects an array literal element that is a reference without a contextual type.
// rule: C11
// phase: P2.3
// tsc: errors TS2345
// expected-error: C11: an inferred type cannot come from a reference
function collect(b: boolean): number {
  if (b) {
    const xs = [b];
    xs.push(false);
    return xs.length;
  }
  return 0;
}

export function main(): void {
  print(`${collect(true)}`);
}
