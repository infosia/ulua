// corpus: accept/a58-async-order
// purpose: Two async tasks interleave in the job order of node.
// rule: C19
// phase: P4.1
// tsc: accepts
// js-comparable: yes
async function value(n: number): Promise<number> {
  return n;
}
async function task(name: string, steps: number): Promise<number> {
  print(`${name} start`);
  let total: number = 0;
  for (let i = 0; i < steps; i++) {
    total += await value(i);
    print(`${name} step ${i}`);
  }
  print(`${name} end`);
  return total;
}
async function adopt(): Promise<number> {
  return value(7);
}
export async function main(): Promise<void> {
  print("main start");
  const a = task("A", 3);
  const b = task("B", 2);
  const c = adopt();
  print("main sync end");
  print(`a=${await a}`);
  print(`b=${await b}`);
  print(`c=${await c}`);
  print("main end");
}
