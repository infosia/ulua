// corpus: accept/a36-never-after-equality
// purpose: Copies and writes number and string references, including after a contradictory comparison.
// rule: C7
// phase: P2.3
// tsc: equality TS2367, TS2322
// js-comparable: yes
function compound(k: number): number {
  if (k === 1) {
    if (k === 2) {
      k *= 2;
    }
  }
  return k;
}

export function main(): void {
  const s = "a";
  if (s === "b") {
    let t = s;
    t = "c";
    print(t);
  }
  let best = Infinity;
  const xs: number[] = [3, 1, 2];
  for (const x of xs) {
    if (x < best) {
      best = x;
    }
  }
  let acc = best;
  acc += 10;
  print(`${compound(1)} ${best} ${acc}`);
}
