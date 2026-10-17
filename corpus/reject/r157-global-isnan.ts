// corpus: reject/r157-global-isnan
// purpose: Rejects the global isNaN, which converts its argument.
// rule: C4
// phase: P7.3
// tsc: accepts
// expected-error: C4: the global isNaN converts its argument; use Number.isNaN
export function main(): void {
  print(`${isNaN(5)}`);
}
