// corpus: trap/t06-string-boundary
// purpose: Traps on a slice that starts inside a UTF-8 code point.
// rule: C5
// phase: P2.4
// tsc: accepts
// js-comparable: yes
// trap: string-boundary
export function main(): void {
  print("ok");
  print("é".slice(1));
}
