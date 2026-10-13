// corpus: trap/t10-unhandled-rejection
// purpose: A rejected promise without a reaction at the end of the drain stops the run.
// rule: C19
// phase: P4.1
// tsc: accepts
// js-comparable: yes
// trap: uncaught-exception
async function fail(): Promise<number> {
  throw new TypeError("lost");
}
export function main(): void {
  print("before");
  const p = fail();
  print("after");
}
