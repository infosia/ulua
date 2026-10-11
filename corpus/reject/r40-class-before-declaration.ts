// corpus: reject/r40-class-before-declaration
// purpose: Rejects a call that reaches a class binding before its declaration.
// rule: C10
// tsc: accepts
// expected-error: C10: the referenced function reads a binding that is not declared yet
function make(): Box {
  return new Box();
}

const early = make();

class Box {
  size: number = 1;
}

export function main(): void {
  print(`${early.size}`);
}
