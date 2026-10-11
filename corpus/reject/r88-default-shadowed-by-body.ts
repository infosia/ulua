// corpus: reject/r88-default-shadowed-by-body
// purpose: Rejects a default that refers to a name the function body declares.
// rule: C11
// phase: P2.1
// tsc: errors TS2373
// expected-error: C11: a default refers to a name that the body declares
function g(): number {
  return 1;
}

function f(a: number = g()): number {
  function g(): number {
    return 2;
  }
  return a + g();
}

export function main(): void {
  print(`${f()}`);
}
