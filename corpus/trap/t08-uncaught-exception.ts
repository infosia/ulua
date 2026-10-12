// corpus: trap/t08-uncaught-exception
// purpose: Stops the run when an exception leaves the exported function.
// rule: C17
// phase: P2.7
// tsc: accepts
// js-comparable: yes
// trap: uncaught-exception
function check(n: number): number {
  if (n < 0) {
    throw new RangeError("negative");
  }
  return n;
}

export function main(): void {
  print(`${check(1)}`);
  print(`${check(-1)}`);
  print("after");
}
