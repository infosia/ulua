export function main(): number {
  let sum = 0;
  for (let i = 0; i < 1000000; i++) {
    let a = 0; let b = 1;
    const m = i % 40;
    for (let j = 0; j < m; j++) { const next = a + b; a = b; b = next; }
    sum = (sum + a) % 4294967296;
  }
  return sum;
}
