// corpus: reject/r63-optional-chain-continuation
// purpose: Rejects a step after the optional step of a chain.
// rule: C2
// phase: P2.5
// tsc: accepts
// expected-error: C2: an optional chain has one optional step and no step after it
type Inner = { g: number };
type Outer = { f: Inner };

function read(a: Outer | null): number {
  return a?.f.g ?? 9;
}

export function main(): void {
  print(`${read(null)}`);
}
