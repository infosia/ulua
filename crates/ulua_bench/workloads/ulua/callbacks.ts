export function main(): number {
  const a: number[] = []; let state = 1;
  for (let i = 0; i < 100000; i++) {
    state = (state * 1664525 + 1013904223) % 4294967296; a.push(state);
  }
  let sum = 0;
  for (let k = 0; k < 20; k++) {
    const mapped = a.map((v: number, i: number): number => (v + i + k) % 4294967296);
    const kept = mapped.filter((v: number, i: number): boolean => (v + i) % 3 !== 0);
    const round = kept.reduce((acc: number, v: number, i: number): number => (acc + v + i) % 4294967296, 0);
    sum = (sum + round) % 4294967296;
  }
  return sum;
}
