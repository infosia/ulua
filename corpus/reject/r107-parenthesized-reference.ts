// corpus: reject/r107-parenthesized-reference
// purpose: Rejects a narrowed member read through a parenthesized object at a D18 position.
// rule: C11
// phase: P2.5
// tsc: errors TS2345
// expected-error: C11: an inferred type cannot come from a narrowable reference
type B = { flag: boolean };

export function main(): void {
  const o: B = { flag: true };
  if (o.flag) {
    const xs = [(o).flag];
    xs.push(false);
    print(`${xs.length}`);
  }
}
