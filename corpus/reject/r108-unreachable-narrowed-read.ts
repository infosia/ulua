// corpus: reject/r108-unreachable-narrowed-read
// purpose: Rejects a read of a nullable const after a return; an unreachable point has no narrowing.
// rule: C16
// phase: P2.5
// tsc: errors TS18047
// expected-error: C16: an unreachable read has the declared type
type B = { v: number };

function mk(): B | null {
  return null;
}

export function main(): void {
  let out = 0;
  const k: B | null = mk();
  if (k != null) {
    return;
    out += k.v;
  }
  print(`${out}`);
}
