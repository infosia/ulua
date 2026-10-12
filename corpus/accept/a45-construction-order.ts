// corpus: accept/a45-construction-order
// purpose: Evaluates constructor arguments, then field initializers in order, then the constructor body.
// rule: C13
// phase: P2.6
// tsc: accepts
// js-comparable: yes
function trace(label: string, v: number): number {
  print(label);
  return v;
}

class Order {
  a: number = trace("field a", 1);
  b: number;
  c: number = trace("field c", 3);

  constructor(x: number, y: number) {
    print("body");
    this.b = x + y;
  }

  sum(): number {
    return this.a + this.b + this.c;
  }
}

class Plain {
  n: number = trace("plain n", 7);
}

export function main(): void {
  const o = new Order(trace("arg x", 10), trace("arg y", 20));
  print(`${o.sum()}`);
  const p = new Plain();
  const q = new Plain();
  print(`${p.n + q.n} ${p === q}`);
}
