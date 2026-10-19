// corpus: trap/t22-invariant-length-at-length
// purpose: A counted loop whose body writes fields only reads an element at the length and traps.
// rule: C6
// phase: P6
// tsc: accepts
// js-comparable: yes
// trap: index-out-of-range
class Cell {
  v: number;
  constructor(v: number) {
    this.v = v;
  }
}

export function main(): void {
  const cs: Cell[] = [new Cell(1), new Cell(2), new Cell(3)];
  let s = 0;
  for (let i = 0; i < 4; i++) {
    const c = cs[i];
    c.v += 1;
    s += c.v;
    print(`${s}`);
  }
}
