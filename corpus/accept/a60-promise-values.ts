// corpus: accept/a60-promise-values
// purpose: Promise values are stored, passed, and awaited more than once.
// rule: C19
// phase: P4.1
// tsc: accepts
// js-comparable: yes
type Job = { name: string; result: Promise<number> };
async function square(n: number): Promise<number> {
  print(`square ${n}`);
  return n * n;
}
async function sum(ps: Promise<number>[]): Promise<number> {
  let total: number = 0;
  for (const p of ps) {
    total += await p;
  }
  return total;
}
async function twice(p: Promise<number>): Promise<number> {
  const a = await p;
  const b = await p;
  return a + b;
}
export async function main(): Promise<void> {
  const ps: Promise<number>[] = [square(1), square(2), square(3)];
  const job: Job = { name: "four", result: square(4) };
  print("started");
  print(`sum ${await sum(ps)}`);
  print(`${job.name} ${await job.result}`);
  print(`twice ${await twice(job.result)}`);
  let later: Promise<number> | null = null;
  later = square(5);
  if (later !== null) {
    print(`later ${await later}`);
  }
}
