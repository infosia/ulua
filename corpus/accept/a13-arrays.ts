// corpus: accept/a13-arrays
// purpose: Reads, writes, appends, and iterates a zero-based array.
// rule: C6
// phase: P2.2
// tsc: accepts
// js-comparable: yes
export function main(): void {
  const xs: number[] = [10, 20, 30];
  print(`${xs[0]} ${xs[2]} ${xs.length}`);
  xs[1] = 21;
  xs[xs.length] = 40;
  xs.push(50);
  print(`${xs.length}`);
  let sum = 0;
  for (const x of xs) {
    sum += x;
  }
  print(`${sum}`);
  const grow: number[] = [1];
  for (const g of grow) {
    if (g < 4) {
      grow.push(g + 1);
    }
  }
  print(`${grow.length}`);
}
