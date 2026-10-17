// corpus: accept/a78-map-order
// purpose: Map insertion order across overwrite, delete, and re-insert, through keys, values, and forEach.
// rule: C6
// phase: P7.4
// tsc: accepts
// js-comparable: yes
export function main(): void {
  const m = new Map<string, number>();
  m.set("a", 1);
  m.set("b", 2);
  m.set("a", 9);
  m.delete("b");
  m.set("b", 3);
  m.set("c", 4);
  let keys = "";
  for (const k of m.keys()) {
    keys = `${keys}${k}`;
  }
  let values = "";
  for (const v of m.values()) {
    values = `${values} ${v}`;
  }
  let pairs = "";
  m.forEach((v: number, k: string): void => {
    pairs = `${pairs} ${k}=${v}`;
  });
  print(`${keys} |${values} |${pairs}`);
  const big = new Map<number, number>();
  for (let i = 0; i < 100; i++) {
    big.set(i, i * i);
  }
  for (let i = 0; i < 100; i += 3) {
    big.delete(i);
  }
  let sum = 0;
  let first = -1;
  for (const k of big.keys()) {
    if (first < 0) {
      first = k;
    }
    sum = sum + k;
  }
  print(`${big.size} ${first} ${sum}`);
}
