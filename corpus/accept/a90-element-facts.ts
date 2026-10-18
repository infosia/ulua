// corpus: accept/a90-element-facts
// purpose: Remainders of array elements and inline callback parameters: confined arrays with integer elements, and each write or use that removes the fact.
// rule: C4
// phase: P6
// tsc: accepts
// js-comparable: yes
function confined(): string {
  const a: number[] = [];
  let state = 7;
  for (let i = 0; i < 50; i++) {
    state = (state * 1103515245 + 12345) % 2147483648;
    a.push(state);
  }
  const m = a.map((v: number, i: number): number => (v + i) % 1000);
  const f = m.filter((v: number, i: number): boolean => (v + i) % 3 !== 0);
  const r = f.reduce((acc: number, v: number, i: number): number => (acc + v + i) % 65536, 0);
  let s = 0;
  for (let i = 0; i < a.length; i++) {
    s = (s + a[i] % 97) % 1000003;
  }
  a[0] %= 10;
  return `${m.length} ${f.length} ${r} ${s} ${a[0]} ${m[49] % 7}`;
}

function fraction(): string {
  const a: number[] = [3, 5];
  a.push(2.5);
  const b: number[] = [4];
  const grow = (): void => {
    b.push(-3);
  };
  grow();
  const c: number[] = [6];
  c[0] = c[0] * -0;
  return `${a[2] % 2} ${b[1] % 2} ${1 / (c[0] % 5)}`;
}

function fill(xs: number[]): void {
  xs.push(0.5);
}

function escaped(): string {
  const a: number[] = [9, 8];
  fill(a);
  const b: number[] = [7];
  const alias = b;
  alias.push(-1);
  return `${a[2] % 3} ${b[1] % 4}`;
}

function accumulator(): string {
  const xs: number[] = [1, 2, 3];
  const half = xs.reduce((acc: number, v: number): number => acc % 4 + v, 0.5);
  const neg = xs.reduce((acc: number, v: number): number => (acc - v) % 5, 0);
  return `${half} ${neg}`;
}

function edges(): string {
  const big: number[] = [9007199254740991, 9007199254740992];
  const r = big.map((v: number, i: number): number => (v + i) % 1000);
  const inf: number[] = [1 / 0];
  return `${r.join(",")} ${big[1] % 7} ${inf[0] % 5}`;
}

export function main(): void {
  print(confined());
  print(fraction());
  print(escaped());
  print(accumulator());
  print(edges());
}
