// corpus: reject/r49-closure-in-for-condition
// purpose: Rejects a closure in the condition of a for head.
// rule: C9
// phase: P2
// tsc: accepts
// expected-error: C9: a closure in a for head is not part of the language
const kept: (() => number)[] = [];

function keep(f: () => number, i: number): boolean {
  kept.push(f);
  return i < 3;
}

export function main(): void {
  for (let i = 0; keep((): number => i, i); i++) {
    print(`${i}`);
  }
  for (const f of kept) {
    print(`${f()}`);
  }
}
