// corpus: reject/r11-var
// purpose: Rejects a var declaration.
// rule: C8
// phase: P1
// tsc: accepts
// expected-error: C8: use let or const
export function main(): void {
  var count = 1;
  print(`${count}`);
}
