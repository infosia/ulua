// corpus: accept/a94-invariant-length
// purpose: Counted loops that read elements while the body writes fields only, calls a function that appends, appends, writes elements, or reassigns the array.
// rule: C6
// phase: P6
// tsc: accepts
// js-comparable: yes
class Cell {
  v: number;
  constructor(v: number) {
    this.v = v;
  }
}

function bump(cs: Cell[]): void {
  cs.push(new Cell(0));
}

export function main(): void {
  const cs: Cell[] = [];
  for (let i = 0; i < 5; i++) {
    cs.push(new Cell(i));
  }
  for (let k = 0; k < 3; k++) {
    for (let i = 0; i < 5; i++) {
      const c = cs[i];
      c.v += c.v + 1;
    }
  }
  let s = 0;
  for (let i = 0; i < 5; i++) {
    s += cs[i].v;
  }
  print(`${s}`);

  const ns: number[] = [1, 2];
  let t = 0;
  for (let i = 0; i < 4; i++) {
    if (ns.length < 4) {
      ns.push(i * 10);
    }
    t += ns[i];
  }
  print(`${t} ${ns.length}`);

  const bs: Cell[] = [new Cell(1)];
  let u = 0;
  for (let i = 0; i < 3; i++) {
    bump(bs);
    u += bs[i + 1].v + i;
  }
  print(`${u} ${bs.length}`);

  const ws: number[] = [7];
  for (let i = 0; i < 3; i++) {
    ws[i + 1] = ws[i] * 2;
  }
  print(ws.join(","));

  const empty: number[] = [];
  let e = 0;
  for (let i = 0; i < 0; i++) {
    e += empty[i];
  }
  print(`${e}`);

  let w = 0;
  for (let k = 0; k < 3; k++) {
    const row: number[] = [k, k + 1, k + 2];
    for (let i = 0; i < 3; i++) {
      w += row[i];
    }
  }
  print(`${w}`);

  let arr: number[] = [1, 2, 3];
  let z = 0;
  for (let i = 0; i < 4; i++) {
    z += arr[i];
    if (i == 0) {
      arr = [10, 20, 30, 40];
    }
  }
  print(`${z}`);
}
