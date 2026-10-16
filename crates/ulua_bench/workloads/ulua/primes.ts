export function main(): number {
  let count = 0;
  for (let n = 2; n <= 200000; n++) {
    let prime = true;
    for (let d = 2; d * d <= n; d++) {
      if (n % d === 0) { prime = false; break; }
    }
    if (prime) { count = (count + 1) % 4294967296; }
  }
  return count;
}
