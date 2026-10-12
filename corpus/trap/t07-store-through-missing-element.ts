// corpus: trap/t07-store-through-missing-element
// purpose: Evaluates the right side of a field store before the trap of its out-of-range element receiver.
// rule: C6, C12
// phase: P2.5
// tsc: accepts
// js-comparable: yes
// trap: index-out-of-range
type T = { f: number };

function h(): number {
  print("h");
  return 1;
}

export function main(): void {
  const xs: T[] = [];
  print("start");
  xs[0].f = h();
  print("after");
}
