// corpus: reject/r112-this-in-function-declaration
// purpose: Rejects this inside a function declaration nested in a method.
// rule: C11
// phase: P2.6
// tsc: errors TS2683
// expected-error: C11: this is not visible in a nested function declaration
class Box {
  v: number = 1;

  read(): number {
    function inner(): number {
      return this.v;
    }
    return inner();
  }
}

export function main(): void {
  print(`${new Box().read()}`);
}
