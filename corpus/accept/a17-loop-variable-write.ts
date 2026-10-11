// corpus: accept/a17-loop-variable-write
// purpose: Writes the loop variable in the body, with continue, and captures it per iteration.
// rule: C9
// phase: P2
// tsc: accepts
// js-comparable: yes
export function main(): void {
  for (let i = 0; i < 5; i++) {
    i += 1;
    print(`${i}`);
  }
  const seen: (() => number)[] = [];
  for (let j = 0; j < 6; j++) {
    if (j === 1) {
      j += 2;
      continue;
    }
    seen.push(() => j);
  }
  for (const read of seen) {
    print(`${read()}`);
  }
}
