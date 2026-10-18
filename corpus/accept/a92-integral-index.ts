// corpus: accept/a92-integral-index
// purpose: Element reads and writes whose index has an integral mark but no fact: down counters, signed sums, rounding calls, signed zeros, and swaps of parameters.
// rule: C6
// phase: P6
// tsc: accepts
// js-comparable: yes
function lastOf(xs: number[], n: number): number {
  return xs[n - 1];
}

function down(xs: number[]): number {
  let s = 0;
  for (let j = xs.length - 1; j >= 0; j--) {
    s = s * 3 + xs[j];
  }
  return s;
}

function rounding(xs: number[]): number {
  return xs[Math.floor(xs.length / 2)] + xs[Math.trunc(1.9)] + xs[Math.round(0.4)] + xs[Math.ceil(0.2)];
}

function zeros(xs: number[]): string {
  let z = 0;
  z = -z;
  const w = 2 * z;
  return `${xs[z]} ${xs[w]} ${1 / z} ${1 / w}`;
}

function swap(xs: number[], i: number, j: number): void {
  const t = xs[i];
  xs[i] = xs[j];
  xs[j] = t;
}

function reverse(xs: number[]): string {
  let i = 0;
  let j = xs.length - 1;
  while (i < j) {
    swap(xs, i, j);
    i++;
    j--;
  }
  return xs.join(",");
}

function signed(xs: number[]): string {
  let k = -2;
  k = Math.abs(k);
  let m = 5;
  m = m - 7;
  m = m * m - 3;
  xs[m] += 100;
  let p = 3;
  p = p % 2 - 1;
  return `${xs[k]} ${xs[m]} ${xs[p]}`;
}

export function main(): void {
  const xs: number[] = [10, 20, 30, 40, 50];
  print(`${lastOf(xs, 5)} ${lastOf(xs, 1)} ${down(xs)} ${rounding(xs)}`);
  print(zeros(xs));
  print(reverse([1, 2, 3, 4, 5, 6]));
  print(signed(xs));
}
