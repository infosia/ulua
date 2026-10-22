// corpus: accept/a95-for-of-shrink
// purpose: Reads the live length when the body shrinks the array, directly, through a function, and in a while test.
// rule: C6
// phase: P2.3
// tsc: accepts
// js-comparable: yes
function drop(ys: number[]): void {
  ys.pop();
}
export function main(): void {
  const xs = [1, 2, 3, 4, 5];
  for (const x of xs) {
    print(`${x} ${xs.length}`);
    if (x === 2) {
      xs.pop();
      xs.shift();
    }
  }
  const ys = [1, 2, 3, 4, 5, 6];
  for (const y of ys) {
    print(`${y}`);
    drop(ys);
  }
  const zs = [1, 2, 3, 4];
  for (const z of zs) {
    print(`${z}`);
    zs.splice(0, 1);
  }
  const ws = [1, 2, 3, 4, 5];
  let i = 0;
  while (i < ws.length) {
    print(`${ws[i]}`);
    ws.pop();
    i++;
  }
  print(`${xs.join()} ${ys.join()} ${zs.join()} ${ws.join()}`);
}
