// corpus: accept/a56-host-call-positions
// purpose: Calls host fixtures in arrows, methods, defaults, and try blocks, with argument order.
// rule: C18
// phase: P3
// grants: hostAdd, hostLog
// tsc: accepts
// js-comparable: yes
declare function hostAdd(a: number, b: number): number;
declare function hostLog(text: string): void;
let n: number = 0;
function next(): number {
  n = n + 1;
  hostLog(`next ${n}`);
  return n;
}
function scaled(x: number, by: number = hostAdd(1, 1)): number {
  return x * by;
}
class Counter {
  total: number = hostAdd(10, 0);
  add(x: number): number {
    this.total = hostAdd(this.total, x);
    return this.total;
  }
}
export function main(): void {
  print(`${hostAdd(next(), next())}`);
  const twice = (x: number): number => hostAdd(x, x);
  print(`${twice(4)}`);
  print(`${scaled(3)}`);
  const c = new Counter();
  print(`${c.add(5)}`);
  try {
    hostLog("in try");
    throw new Error("e");
  } catch (e) {
    if (e instanceof Error) {
      hostLog(e.message);
    }
  } finally {
    hostLog("finally");
  }
  const xs: number[] = [1, 2, 3];
  print(`${xs.map((x: number): number => hostAdd(x, 100)).join(",")}`);
}
