// corpus: reject/r111-conditional-test-narrowing
// purpose: Rejects a read narrowed only through a conditional expression used as a test.
// rule: C16
// phase: P2.5
// tsc: errors TS18047
// expected-error: C16: a conditional test narrows nothing
type R = { v: number };

function mk(n: number): R | null {
  return n > 0 ? { v: n } : null;
}

export function main(): void {
  const x = mk(1);
  const c = mk(3) != null;
  let out = 0;
  if (c ? x != null : false) {
    out += x.v;
  }
  print(`${out}`);
}
