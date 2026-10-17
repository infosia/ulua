// corpus: reject/r161-map-for-of
// purpose: Rejects for...of over a Map, whose element is a [key, value] tuple.
// rule: C6
// phase: P7.4
// tsc: accepts
// expected-error: C6: for...of over a Map needs a tuple type; iterate keys() or values()
export function main(): void {
  const m = new Map<string, number>();
  for (const e of m) {
    print(`${e[1]}`);
  }
}
