export function main(): number {
  let sum = 0;
  for (let row = 0; row < 400; row++) {
    const cy = row / 400 * 2 - 1;
    for (let col = 0; col < 400; col++) {
      const cx = col / 400 * 3 - 2;
      let x = 0; let y = 0; let count = 0;
      while (count < 255 && x * x + y * y < 4) {
        const next = x * x - y * y + cx;
        y = 2 * x * y + cy; x = next; count++;
      }
      sum = (sum + count) % 4294967296;
    }
  }
  return sum;
}
