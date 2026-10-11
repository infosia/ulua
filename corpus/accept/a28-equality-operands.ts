// corpus: accept/a28-equality-operands
// purpose: Compares a wide operand with a literal, including after a test that does not narrow.
// rule: C7
// phase: P1
// tsc: accepts
// js-comparable: yes
function pick(n: number): string {
  if (n === 1 || n === 2) {
    return "low";
  }
  if (n !== 3) {
    return "other";
  }
  return "three";
}

function count(limit: number): number {
  let hits = 0;
  for (let i = 0; i < limit; i++) {
    if (i === 3) {
      continue;
    }
    if (i === 7) {
      hits = hits + 10;
    }
    hits = hits + 1;
  }
  return hits;
}

export function main(): void {
  print(`${pick(2)} ${pick(5)} ${pick(3)}`);
  print(`${count(10)}`);
  const k = 2;
  print(`${(-k) === -2} ${(1 + 1) === k}`);
}
