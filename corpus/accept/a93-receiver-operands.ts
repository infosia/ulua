// corpus: accept/a93-receiver-operands
// purpose: Field and element receivers that are locals, captured bindings, and module bindings, with a call in the index or the right side.
// rule: C12
// phase: P6
// tsc: accepts
// js-comparable: yes
class P {
  x: number;
  y: number;
  constructor(x: number, y: number) {
    this.x = x;
    this.y = y;
  }
}

let shared: P = new P(1, 2);

function swapShared(): number {
  shared = new P(100, 200);
  return 5;
}

export function main(): void {
  const p = new P(3, 4);
  p.x += p.y * 2;
  p.y = p.x - 1;
  print(`${p.x} ${p.y}`);

  const xs: number[] = [1, 2, 3];
  xs[1] = xs[0] + xs[2];
  xs[2] += xs[1];
  print(`${xs[0]} ${xs[1]} ${xs[2]}`);

  const old = shared;
  shared.x += swapShared();
  print(`${old.x} ${shared.x} ${shared.y}`);

  let q = new P(7, 8);
  const swapQ = (): number => {
    q = new P(70, 80);
    return 1;
  };
  const oldQ = q;
  q.y += swapQ();
  print(`${oldQ.y} ${q.y}`);

  let ys: number[] = [5, 6, 7];
  const swapYs = (): number => {
    ys = [50, 60, 70];
    return 2;
  };
  print(`${ys[swapYs()]} ${ys[0]}`);

  let ws: number[] = [1, 1, 1];
  const swapWs = (): number => {
    ws = [9, 9, 9];
    return 3;
  };
  const oldWs = ws;
  ws[0] = swapWs();
  print(`${oldWs[0]} ${ws[0]}`);

  let r = new P(0, 0);
  const swapR = (): P => {
    r = new P(-1, -1);
    return new P(40, 41);
  };
  const oldR = r;
  r.x = swapR().y;
  print(`${oldR.x} ${r.x}`);
}
