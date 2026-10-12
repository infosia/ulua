// corpus: accept/a31-array-evaluation-order
// purpose: Pins the evaluation order of element reads, element writes, compound writes, and push.
// rule: C12
// phase: P2.2
// tsc: accepts
// js-comparable: yes
let xs: number[] = [1, 2, 3];
const ys: number[] = [7, 8, 9];

function swap(): number {
  xs = ys;
  return 0;
}

function grow(): number {
  xs.push(4);
  return 40;
}

export function main(): void {
  const a = xs[swap()];
  print(`${a} ${xs[0]}`);
  xs = [1, 2, 3];
  xs[3] = grow();
  print(`${xs.length} ${xs[3]}`);
  const zs: number[] = [1];
  zs.push(zs.length);
  print(`${zs.length} ${zs[1]}`);
  const t: number[] = [5];
  function bump(): number {
    t.push(6);
    return 2;
  }
  t[0] += bump();
  print(`${t[0]} ${t.length}`);
  for (const x of t) {
    if (x === 6) {
      print("six");
    }
  }
}
