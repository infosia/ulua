// corpus: reject/r110-logical-literal-loop-exit
// purpose: Rejects a narrowed read after a loop whose logical test has no false edge.
// rule: C16
// phase: P2.5
// tsc: errors TS18047
// expected-error: C16: an unreachable read has the declared type
type R = { v: number };

function mk(n: number): R | null {
  return n > 0 ? { v: n } : null;
}

export function main(): void {
  const x = mk(1);
  const c = mk(3) != null;
  let out = 0;
  if (x == null) {
    return;
  }
  while (true || c) {
    out += 1;
    if (out > 3) {
      print(`${out}`);
      return;
    }
  }
  out += x.v;
  print(`${out}`);
}
