function place(cols: number[], row: number): number {
  if (row === 11) { return 1; }
  let count = 0;
  for (let col = 0; col < 11; col++) {
    let safe = true;
    for (let r = 0; r < row; r++) {
      if (cols[r] === col || cols[r] - col === row - r || col - cols[r] === row - r) { safe = false; break; }
    }
    if (safe) { cols[row] = col; count = (count + place(cols, row + 1)) % 4294967296; }
  }
  return count;
}
export function main(): number {
  const cols: number[] = [];
  for (let i = 0; i < 11; i++) { cols.push(0); }
  return place(cols, 0);
}
