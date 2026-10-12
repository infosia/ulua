// corpus: accept/a41-never-member
// purpose: Reads a member of a string that a contradictory comparison narrows to never in tsc.
// rule: C7
// phase: P2.4
// tsc: exception TS2367, TS2339
// js-comparable: yes
export function main(): void {
  const s = "a";
  if (s === "b") {
    print(`${s.length}`);
  }
  print(`${s.length}`);
}
