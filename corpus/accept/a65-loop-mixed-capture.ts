// corpus: accept/a65-loop-mixed-capture
// purpose: A for head with one captured and one uncaptured binding; the body writes both, uses continue and break.
// rule: C9
// phase: P4.3
// tsc: accepts
// js-comparable: yes
export function main(): void {
  const readers: (() => number)[] = [];
  for (let i = 0, n = 0; i < 8; i++) {
    n = n + i;
    if (i == 2) {
      i = i + 1;
      continue;
    }
    if (n > 20) {
      break;
    }
    readers.push(() => i * 10);
  }
  let total = 0;
  for (const read of readers) {
    total = total + read();
    print(`${read()}`);
  }
  let steps = 0;
  for (let k = 0; k < 4; k++) {
    steps = steps + k;
  }
  print(`${total} ${steps}`);
}
