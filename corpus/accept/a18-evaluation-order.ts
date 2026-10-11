// corpus: accept/a18-evaluation-order
// purpose: Keeps left-to-right evaluation when a later operand changes an earlier one.
// rule: C12
// phase: P2
// tsc: accepts
type Box = { f: number };

let n = 0;
let total = 10;
let current: Box = { f: 0 };
const other: Box = { f: 0 };

function inc(): number {
  n += 1;
  return n;
}

function bump(): number {
  total = 100;
  return 1;
}

function swap(): number {
  current = other;
  return 9;
}

let xs: number[] = [1, 2];
const ys: number[] = [1];

function replaceXs(): number {
  xs = [7, 8];
  return 0;
}

function grow(): number {
  ys.push(5);
  return 9;
}

const ws: number[] = [1];

function growWs(): number {
  ws.push(5);
  return 9;
}

class K {
  v: number;

  constructor(v: number) {
    this.v = v;
  }

  m(a: number): number {
    return this.v + a;
  }
}

let holder = new K(10);

function replaceHolder(): number {
  holder = new K(20);
  return 1;
}

function trace(label: string, v: number): number {
  print(label);
  return v;
}

export function main(): void {
  print(`${n + inc()}`);
  total += bump();
  print(`${total}`);
  const first = current;
  current.f = swap();
  print(`${first.f} ${other.f}`);
  let k = 1;
  const same = k === ((): number => {
    k = 2;
    return 2;
  })();
  print(`${same}`);
  print(`${trace("a", 1) - trace("b", 2) * trace("c", 3)}`);
  print(`${xs[replaceXs()]} ${xs[0]}`);
  ys[2] = grow();
  for (const y of ys) {
    print(`${y}`);
  }
  print(`${holder.m(replaceHolder())}`);
  let zs: number[] = [1, 2];
  for (const z of zs) {
    zs = [7, 8, 9];
    print(`${z}`);
  }
  print(`${zs.length}`);
  ws.push(growWs());
  for (const w of ws) {
    print(`${w}`);
  }
}
