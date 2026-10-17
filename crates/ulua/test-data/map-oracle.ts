// Shared input program for the Node oracle and the product VM.
export function main(seed: number): string {
  let random = seed;
  const m = new Map<number, number>();
  let s = new Set<number>();
  let result = "";
  for (let step = 0; step < 32; step++) {
    random = (random * 1664525 + 1013904223) % 4294967296;
    const op = random % 12;
    random = (random * 1664525 + 1013904223) % 4294967296;
    let key = random % 12;
    if (key === 0) { key = NaN; }
    if (key === 1) { key = -0; }
    if (key === 2) { key = Infinity; }
    if (key === 3) { key = -Infinity; }
    const value = random % 100;
    if (op === 0 || op === 1 || op === 2) {
      m.set(key, value); s.add(key);
    } else if (op === 3) {
      result = `${result}d${m.delete(key)},${s.delete(key)};`;
    } else if (op === 4) {
      result = `${result}h${m.has(key)},${s.has(key)};`;
    } else if (op === 5) {
      result = `${result}g${m.get(key) ?? -1},${m.getOr(key, -2)};`;
    } else if (op === 6) {
      m.clear(); s.clear();
    } else if (op === 7) {
      const copy = new Map<number, number>(m);
      copy.set(key, value);
      result = `${result}c${copy.size},${m.size};`;
      s = new Set<number>(s);
    } else if (op === 8) {
      s = new Set<number>([key, value, key, -0, NaN]);
    } else if (op === 9) {
      m.delete(key); m.set(key, value);
      s.delete(key); s.add(key);
    } else if (op === 10) {
      m.set(key, value).set(key, value + 1);
      s.add(key).add(key);
    } else {
      let total = 0;
      m.forEach((v: number, k: number): void => { total = total + v; });
      let count = 0;
      s.forEach((k: number): void => { count++; });
      result = `${result}e${total},${count};`;
    }
    result = `${result}[${m.size},${s.size}|`;
    for (const k of m.keys()) { result = `${result}${k},`; }
    result = `${result}|`;
    for (const v of m.values()) { result = `${result}${v},`; }
    result = `${result}|`;
    for (const k of s) { result = `${result}${k},`; }
    result = `${result}]`;
  }
  return result;
}
