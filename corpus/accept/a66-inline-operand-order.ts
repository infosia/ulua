// corpus: accept/a66-inline-operand-order
// purpose: Reads of captured and module-level bindings beside calls that write them keep the JavaScript read point.
// rule: C12
// phase: P4.3
// tsc: accepts
// js-comparable: yes
let shared = 1;

function bump(): number {
  shared = shared + 10;
  return shared;
}

export function main(): void {
  const a = shared + bump();
  print(`${a}`);
  let n = 2;
  const grow = (): number => {
    n = n * 3;
    return n;
  };
  const b = n + grow() + n;
  print(`${b} ${n}`);
  const f = (x: number, y: number, z: number): number => x * 100 + y * 10 + z;
  print(`${f(n % 10, grow() % 10, n % 10)}`);
  let plain = 4;
  const c = plain * 2 + bump() - plain;
  print(`${c} ${shared}`);
  let w = 0;
  while (w < 3 && w != bump() - shared) {
    w = w + 1;
  }
  print(`${w} ${shared}`);
  if (n > bump()) {
    print("n");
  } else {
    print(`${shared}`);
  }
}
