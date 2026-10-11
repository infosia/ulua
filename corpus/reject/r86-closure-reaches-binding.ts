// corpus: reject/r86-closure-reaches-binding
// purpose: Rejects a call of a declaration whose returned closure reads a binding not declared yet.
// rule: C10
// phase: P2.1
// tsc: accepts
// expected-error: C10: the called function reaches a binding that is not declared yet
export function main(): void {
  const r = mk();
  print(`${r()}`);
  let x = 1;
  function mk(): () => number {
    return (): number => x;
  }
}
