// corpus: reject/r25-let-no-initializer
// purpose: Rejects a let declaration with no initializer.
// rule: C8
// tsc: accepts
// expected-error: C8: a let declaration needs an initializer
export function main(): void {
  let count: number;
  count = 1;
  print(`${count}`);
}
