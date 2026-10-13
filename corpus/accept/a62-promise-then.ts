// corpus: accept/a62-promise-then
// purpose: then, catch, and finally run their callbacks in the job order of node.
// rule: C19
// phase: P4.2
// tsc: accepts
// js-comparable: yes
async function value(n: number): Promise<number> {
  return n;
}
async function fail(message: string): Promise<number> {
  throw new RangeError(message);
}
async function ticks(name: string, n: number): Promise<void> {
  for (let i = 0; i < n; i++) {
    await value(i);
    print(`${name} tick ${i}`);
  }
}
export async function main(): Promise<void> {
  const t = ticks("t", 6);
  const a = value(1).then((v: number): number => {
    print(`then ${v}`);
    return v + 1;
  });
  const b = fail("bad").catch((e: unknown): number => {
    if (e instanceof RangeError) {
      print(`catch ${e.message}`);
    }
    return 0;
  });
  const c = value(2).finally((): void => {
    print("finally");
  });
  const d = value(3).then(async (v: number): Promise<string> => {
    print(`async then ${v}`);
    return `s${await value(v * 10)}`;
  });
  print("sync end");
  print(`a=${await a} b=${await b} c=${await c} d=${await d}`);
  await t;
}
