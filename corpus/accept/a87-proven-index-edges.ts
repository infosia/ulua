// corpus: accept/a87-proven-index-edges
// purpose: Element reads and writes whose index has an integer fact: literals, counters, sums, remainders, appends, and compound writes.
// rule: C6
// phase: P6
// tsc: accepts
// js-comparable: yes
function reads(): string {
  const xs: number[] = [10, 20, 30, 40, 50];
  let sum = 0;
  for (let i = 0; i < xs.length; i++) {
    sum += xs[i];
  }
  let k = 0;
  let pairs = 0;
  while (k + 1 < xs.length) {
    pairs += xs[k + 1] - xs[k];
    k++;
  }
  return `${xs[0]} ${xs[4]} ${sum} ${pairs} ${xs[k % 3]} ${xs[2 * 2]}`;
}

function appends(): string {
  const ys: number[] = [];
  let n = 0;
  while (n < 4) {
    ys[n] = n * n;
    n++;
  }
  ys[ys.length] = 99;
  let m = 0;
  m = 5;
  ys[m] = 7;
  return `${ys.length} ${ys.join(",")}`;
}

function compound(): string {
  const zs: number[] = [1, 2, 3];
  const ws: string[] = ["a", "b", "c"];
  for (let i = 0; i < 3; i++) {
    zs[i] += i * 10;
    zs[i] *= 2;
    ws[i] += `${zs[i]}`;
  }
  return `${zs.join(",")} ${ws.join(",")}`;
}

function mixed(xs: number[], j: number): string {
  let i = 0;
  i++;
  const half = 1.5;
  return `${xs[i]} ${xs[j]} ${xs[half - 0.5]} ${xs[-0]}`;
}

function shared(): string {
  const xs: number[] = [3, 1, 4, 1, 5];
  let i = 0;
  const step = (): void => {
    i += 2;
  };
  let out = "";
  while (i < xs.length) {
    out += `${xs[i]}`;
    step();
  }
  return out;
}

export function main(): void {
  print(reads());
  print(appends());
  print(compound());
  print(mixed([7, 8, 9], 2));
  print(shared());
}
