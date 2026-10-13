// corpus: reject/r130-declare-const
// purpose: Rejects a declare form.
// rule: C18
// phase: P3
// tsc: accepts
// expected-error: C18: a script declares no host function
declare const limit: number;
export function main(): void {
  print(`${limit}`);
}
