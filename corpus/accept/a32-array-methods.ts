// corpus: accept/a32-array-methods
// purpose: Uses the array methods without callbacks, with negative and fractional positions.
// rule: C6
// phase: P2.3
// tsc: accepts
// js-comparable: yes
export function main(): void {
  const xs: number[] = [3, 1, 4, 1, 5, 9, 2, 6];
  print(`${xs.indexOf(1)} ${xs.indexOf(1, 2)} ${xs.indexOf(1, -3)} ${xs.lastIndexOf(1)} ${xs.lastIndexOf(1, 2)} ${xs.indexOf(7)}`);
  const ns: number[] = [0 / 0, -0, 1];
  print(`${ns.indexOf(0 / 0)} ${ns.includes(0 / 0)} ${ns.indexOf(0)} ${ns.includes(0, 2)}`);
  print(xs.join());
  print(xs.join(" - "));
  const fs: number[] = [0.1, 1e21, -0, 1 / 0];
  print(fs.join("|"));
  const bs: boolean[] = [true, false];
  print(bs.join());
  print(`${xs.slice(2, 5).join()} [${xs.slice(-3).join()}] [${xs.slice(1.9, -5).join()}] [${xs.slice(5, 2).join()}]`);
  print(xs.concat([7, 8]).join());
  const ys: number[] = [1, 2, 3, 4, 5];
  ys.fill(0, 1, 3);
  print(ys.join());
  ys.reverse();
  print(ys.join());
  const removed = ys.splice(1, 2);
  print(`${removed.join()} / ${ys.join()}`);
  print(`${ys.unshift(9)} ${ys.join()}`);
  const zs: number[] = [1, 2, 3, 4, 5];
  zs.copyWithin(0, 3);
  print(zs.join());
  print(`${zs.at(-1)} ${zs.at(0)} ${zs.pop()} ${zs.shift()} ${zs.join()}`);
  zs.length = 0;
  print(`${zs.length}`);
}
