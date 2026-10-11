// corpus: reject/r56-hoisted-chain
// purpose: Rejects a call that reaches a binding through a chain of three function declarations.
// rule: C10
// tsc: accepts
// expected-error: C10: the referenced function reads a binding that is not declared yet
const x: number = h();
const b: number = 1;

function g(): number {
  return b;
}

function f(): number {
  return g();
}

function h(): number {
  return f();
}

export function main(): void {
  print(`${x}`);
}
