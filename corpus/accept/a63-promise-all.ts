// corpus: accept/a63-promise-all
// purpose: Promise.all gives the values in source order and rejects with the first rejection in settle order.
// rule: C19
// phase: P4.2
// tsc: accepts
// js-comparable: yes
async function delayed(n: number, steps: number): Promise<number> {
  for (let i = 0; i < steps; i++) {
    await later();
  }
  print(`settle ${n}`);
  return n;
}
async function later(): Promise<void> {}
async function failAfter(steps: number, message: string): Promise<number> {
  for (let i = 0; i < steps; i++) {
    await later();
  }
  throw new Error(message);
}
export async function main(): Promise<void> {
  const xs: number[] = await Promise.all([delayed(1, 3), delayed(2, 1), delayed(3, 2)]);
  print(`all ${xs.join(",")}`);
  const ps: Promise<number>[] = [1, 2, 3].map((n: number): Promise<number> => delayed(n * 10, 3 - n));
  const ys = await Promise.all(ps);
  print(`mapped ${ys.join(",")}`);
  const words = await Promise.all(["a", "b"].map(async (w: string): Promise<string> => `${w}!`));
  print(`words ${words.join(" ")}`);
  try {
    await Promise.all([delayed(4, 2), failAfter(1, "first"), failAfter(3, "second")]);
  } catch (e) {
    if (e instanceof Error) {
      print(`caught ${e.message}`);
    }
  }
  print("done");
}
