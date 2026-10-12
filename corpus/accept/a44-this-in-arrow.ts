// corpus: accept/a44-this-in-arrow
// purpose: Reads and writes the receiver from arrows inside methods, and returns this.
// rule: C13
// phase: P2.6
// tsc: accepts
// js-comparable: yes
class Acc {
  scale: number;
  total: number = 0;

  constructor(scale: number) {
    this.scale = scale;
  }

  addAll(xs: number[]): Acc {
    const ys = xs.map((v: number): number => v * this.scale);
    ys.forEach((y: number): void => {
      this.total += y;
    });
    return this;
  }

  adder(): (n: number) => number {
    return (n: number): number => {
      this.total += n;
      return this.total;
    };
  }
}

function same(a: Acc, b: Acc): boolean {
  return a === b;
}

export function main(): void {
  const a = new Acc(2);
  const b = a.addAll([1, 2, 3]).addAll([4]);
  print(`${a.total} ${same(a, b)}`);
  const add = a.adder();
  add(5);
  print(`${add(1)} ${a.total}`);
  const c = new Acc(1);
  print(`${same(a, c)}`);
}
