// corpus: reject/r103-for-update-after-continue
// purpose: Rejects a for update that reads a nullable const a continue edge does not narrow.
// rule: C16
// phase: P2.5
// tsc: errors TS18047
// expected-error: C16: the for update joins every continue edge
type Box = { value: number };

function get(v: boolean): Box | null {
  return v ? { value: 1 } : null;
}

export function main(): void {
  const x: Box | null = get(false);
  let s = 0;
  for (let i = 0; i < 3; s += x.value) {
    i += 1;
    if (x == null) {
      continue;
    }
  }
  print(`${s}`);
}
