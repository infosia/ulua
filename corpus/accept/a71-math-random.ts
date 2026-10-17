// corpus: accept/a71-math-random
// purpose: Math.random draws the seeded xoshiro256++ sequence of the default seed.
// rule: C4
// phase: P7
// tsc: accepts
// js-comparable: no C4
export function main(): void {
  let line = "";
  for (let i = 0; i < 5; i++) {
    line = `${line} ${Math.random()}`;
  }
  print(line);
  let low = 0;
  let sum = 0;
  for (let i = 0; i < 1000; i++) {
    const r = Math.random();
    if (r < 0.5) {
      low++;
    }
    sum = sum + Math.floor(r * 100);
  }
  print(`${low} ${sum}`);
}
