// corpus: accept/a07-loops
// purpose: Runs while and counted for loops with break and continue.
// rule: C3, C9
// phase: P1
// tsc: accepts
export function main(): void {
  let total = 0;
  for (let i = 0; i < 10; i++) {
    if (i === 3) {
      continue;
    }
    if (i === 7) {
      break;
    }
    total += i;
  }
  print(`${total}`);
  let n = 1;
  while (n < 100) {
    n *= 3;
  }
  print(`${n}`);
}
