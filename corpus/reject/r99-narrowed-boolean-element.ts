// corpus: reject/r99-narrowed-boolean-element
// purpose: Rejects a boolean element read as an array literal element without a contextual type.
// rule: C11
// phase: P2.3
// tsc: errors TS2345
// expected-error: C11: an inferred type cannot come from a boolean reference
function count(i: number): number {
  const bs: boolean[] = [true];
  if (bs[i]) {
    const ys = [bs[i]];
    ys.push(false);
    return ys.length;
  }
  return 0;
}

export function main(): void {
  print(`${count(0)}`);
}
