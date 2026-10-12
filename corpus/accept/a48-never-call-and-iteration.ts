// corpus: accept/a48-never-call-and-iteration
// purpose: Calls and iterates nullable references that tsc narrows to never after an assignment of null.
// rule: C16
// phase: P2.5
// tsc: exception TS2349, TS2488
// js-comparable: yes
export function main(): void {
  let k: ((m: number) => number) | null = null;
  let xs: number[] | null = null;
  let s = 0;
  if (k != null) {
    s = s + k(1);
  }
  if (xs != null) {
    for (const x of xs) {
      s = s + x;
    }
  }
  k = (m: number): number => m * 2;
  xs = [3, 4];
  if (k != null) {
    s = s + k(5);
  }
  if (xs != null) {
    for (const x of xs) {
      s = s + x;
    }
  }
  print(`${s}`);
}
