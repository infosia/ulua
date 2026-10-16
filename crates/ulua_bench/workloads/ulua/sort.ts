function quick(a: number[], lo: number, hi: number): void {
  let i = lo; let j = hi;
  const pivot = a[(lo + hi - (lo + hi) % 2) / 2];
  while (i <= j) {
    while (a[i] < pivot) { i++; }
    while (a[j] > pivot) { j--; }
    if (i <= j) { const value = a[i]; a[i] = a[j]; a[j] = value; i++; j--; }
  }
  if (lo < j) { quick(a, lo, j); }
  if (i < hi) { quick(a, i, hi); }
}
export function main(): number {
  const a: number[] = []; let state = 1;
  for (let i = 0; i < 100000; i++) {
    state = (state * 1664525 + 1013904223) % 4294967296; a.push(state);
  }
  quick(a, 0, 99999);
  let sum = 0;
  for (let i = 0; i < 100000; i += 97) { sum = (sum + a[i]) % 4294967296; }
  return sum;
}
