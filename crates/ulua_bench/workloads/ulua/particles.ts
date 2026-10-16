class Particle {
  x: number; y: number; vx: number; vy: number;
  constructor(i: number) { this.x = i % 100; this.y = i % 71; this.vx = i % 13 + 1; this.vy = i % 17 + 1; }
}
export function main(): number {
  const a: Particle[] = [];
  for (let i = 0; i < 20000; i++) { a.push(new Particle(i)); }
  for (let k = 0; k < 200; k++) {
    for (let i = 0; i < 20000; i++) { const p = a[i]; p.x += p.vx * 0.125; p.y += p.vy * 0.125; }
  }
  let sum = 0;
  for (let i = 0; i < 20000; i++) { const p = a[i]; sum = (sum + (p.x * 8 - (p.x * 8) % 1) + (p.y * 8 - (p.y * 8) % 1)) % 4294967296; }
  return sum;
}
