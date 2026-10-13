// corpus: reject/r142-narrowing-across-await
// purpose: Rejects a read of an enclosing let after an await in a nested body, where the enclosing function can assign it.
// rule: C16
// phase: P4.1
// tsc: accepts
// expected-error: C16: an await in a nested body ends the narrowing of an enclosing binding
async function value(n: number): Promise<number> {
  return n;
}
export async function main(): Promise<void> {
  let x: number | null = 1;
  const f = async (): Promise<number> => {
    if (x != null) {
      await value(0);
      return x + 1;
    }
    return 0;
  };
  const p = f();
  x = null;
  print(`${await p}`);
}
