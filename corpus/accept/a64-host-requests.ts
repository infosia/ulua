// corpus: accept/a64-host-requests
// purpose: Async host requests settle one per host-started run, and their jobs run before the next settle.
// rule: C19
// phase: P4.3
// grants: hostLater, hostLog
// tsc: accepts
// js-comparable: yes
async function task(name: string, a: number, b: number): Promise<number> {
  hostLog(`${name} start`);
  const x = await hostLater(a);
  hostLog(`${name} got ${x}`);
  const y = await hostLater(b);
  hostLog(`${name} got ${y}`);
  return x + y;
}
export async function main(): Promise<void> {
  const first = task("A", 1, 2);
  const second = task("B", 10, 20);
  const both = Promise.all([first, second]);
  const doubled = hostLater(5).then((v: number): number => v * 2);
  print("main waits");
  const totals = await both;
  print(`totals ${totals.join(",")}`);
  print(`doubled ${await doubled}`);
}
