// corpus: reject/r106-optional-chain-reference
// purpose: Rejects a narrowed optional-chain member read at a D18 position; tsc narrows o?.s as o.s.
// rule: C11
// phase: P2.5
// tsc: errors TS2322
// expected-error: C11: an inferred type cannot come from a narrowable reference
type B = { s: string | null };

function get(): B | null {
  return { s: "a" };
}

export function main(): void {
  const o = get();
  if (o != null && o.s != null) {
    let y = o?.s ?? null;
    y = null;
    print(`${y == null}`);
  }
}
