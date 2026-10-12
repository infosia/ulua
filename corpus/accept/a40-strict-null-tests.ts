// corpus: accept/a40-strict-null-tests
// purpose: Tests null with === and !== as with == and !=; no accepted form gives undefined (D16).
// rule: C2
// phase: P2.5
// tsc: accepts
// js-comparable: yes
type Box = { value: number };

function first(xs: Box[]): Box | null {
  return xs.length > 0 ? xs[0] : null;
}

export function main(): void {
  const a = first([{ value: 3 }]);
  const b = first([]);
  print(`${a === null} ${a !== null} ${b === null} ${b !== null} ${b == null}`);
  if (a !== null) {
    print(`${a.value}`);
  }
}
