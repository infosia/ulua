// corpus: reject/r81-legacy-octal
// purpose: Rejects a legacy octal literal, an early error of strict-mode module code.
// rule: C11
// phase: P1
// tsc: errors TS1121
// expected-error: C11: a legacy octal literal is not strict-mode code
export function main(): void {
  print(`${010}`);
}
