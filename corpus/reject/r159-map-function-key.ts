// corpus: reject/r159-map-function-key
// purpose: Rejects a function type as a Map key type; C7 defines no equality for functions.
// rule: C6
// phase: P7.4
// tsc: accepts
// expected-error: C6: a Map or Set key type has a defined equality
export function main(): void {
  const m = new Map<() => number, number>();
  print(`${m.size}`);
}
