// corpus: accept/a91-parameter-facts
// purpose: Remainders and element reads of parameters: direct functions, and each call, value use, default, or assignment that removes the fact.
// rule: C4
// phase: P6
// tsc: accepts
// js-comparable: yes
function rem(x: number, d: number): number {
  return x % d;
}

function frac(x: number): number {
  return x % 2;
}

function zero(z: number): number {
  return 1 / (z % 5);
}

function viaValue(n: number): number {
  return n % 3;
}

function shift(n: number): number {
  n = n - 10;
  return n % 4;
}

function dflt(n: number, m: number = -5): number {
  return (n + m) % 3;
}

function at(xs: number[], i: number): number {
  return xs[i];
}

function count(xs: number[], lo: number, hi: number): number {
  if (lo >= hi) {
    return 0;
  }
  const mid = lo + (hi - lo - (hi - lo) % 2) / 2;
  return (xs[lo] % 10) + count(xs, lo + 1, hi) + (mid % 1);
}

function unused(n: number): number {
  return n % 9;
}

export function exported(n: number): number {
  return n % 6;
}

function nested(): string {
  function inner(k: number): number {
    return k % 4;
  }
  const run = (): number => inner(-7);
  return `${inner(9)} ${run()}`;
}

export function main(): void {
  print(`${rem(10, 7)} ${rem(9007199254740992, 7)} ${rem(-3, 7)}`);
  print(`${frac(5)} ${frac(2.5)}`);
  print(`${zero(10)} ${zero(-0)}`);
  print(`${viaValue(7)} ${rem(0 / 0, 3)} ${rem(9007199254740993, 1000)}`);
  const g: (n: number) => number = viaValue;
  print(`${g(-4)}`);
  print(`${shift(3)} ${shift(15)}`);
  print(`${dflt(1)} ${dflt(1, 4)}`);
  const xs: number[] = [13, 25, 37, 49];
  print(`${at(xs, 0)} ${at(xs, 3)} ${count(xs, 0, 4)}`);
  print(`${exported(13)} ${nested()}`);
}
