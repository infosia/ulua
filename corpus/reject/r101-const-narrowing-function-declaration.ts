// corpus: reject/r101-const-narrowing-function-declaration
// purpose: Rejects a read of a narrowed nullable const inside a nested function declaration.
// rule: C16
// phase: P2.5
// tsc: errors TS18047
// expected-error: C16: the narrowing of a const does not continue into a function declaration
type Box = { value: number };

function read(p: Box | null): number {
  const x: Box | null = p;
  if (x === null) {
    return 0;
  }
  function inner(): number {
    return x.value;
  }
  return inner();
}

export function main(): void {
  print(`${read({ value: 3 })}`);
}
