// corpus: reject/r82-strict-reserved-binding
// purpose: Rejects a binding named by a strict-mode reserved word.
// rule: C11
// phase: P1
// tsc: errors TS1215
// expected-error: C11: a strict-mode module cannot bind this name
export function main(): void {
  let arguments = 1;
  print(`${arguments}`);
}
