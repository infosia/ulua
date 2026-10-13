// corpus: reject/r125-unreachable-arrow-null
// purpose: Rejects an arrow function after a return.
// rule: C16
// phase: P2.7
// tsc: errors TS18047
// expected-error: C16: an arrow function at a point that no edge reaches
class B {
  v: number = 2;
}
function f(): number {
  return 0;
  print(`${((b: B | null): number => {
    if (b !== null) {
      return b.v;
    }
    return 1;
  })(null)}`);
}
export function main(): void {
  print(`${f()}`);
}
