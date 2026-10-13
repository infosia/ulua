// corpus: reject/r130-declare-const
// purpose: Rejects a declare form other than declare function.
// rule: C18
// phase: P3
// tsc: accepts
// expected-error: C18: only declare function is accepted
declare const limit: number;
export function main(): void {
  print(`${limit}`);
}
